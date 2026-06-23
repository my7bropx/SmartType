package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/getlantern/systray"
	daemon "github.com/sevlyar/go-daemon"
)

var (
	signalFlag = flag.String("s", "", "send signal to daemon (stop, reload)")
	daemonize  = flag.Bool("d", false, "run as daemon")
)

func main() {
	flag.Parse()

	cntxt := &daemon.Context{
		PidFileName: "/tmp/smarttype.pid",
		PidFilePerm: 0644,
		LogFileName: "/tmp/smarttype.log",
		LogFilePerm: 0640,
		WorkDir:     "/",
		Umask:       027,
	}

	// Signal-forwarding mode: send a control signal to the running daemon.
	if len(*signalFlag) > 0 {
		d, err := cntxt.Search()
		if err != nil {
			log.Fatalf("Cannot find daemon: %s", err)
		}
		switch *signalFlag {
		case "stop":
			d.Signal(syscall.SIGTERM)
			fmt.Println("SmartType stopped")
		case "reload":
			d.Signal(syscall.SIGHUP)
			fmt.Println("SmartType config reloaded")
		default:
			fmt.Println("Unknown signal:", *signalFlag)
		}
		return
	}

	// Daemonize if requested.
	if *daemonize {
		d, err := cntxt.Reborn()
		if err != nil {
			log.Fatal("Cannot daemonize: ", err)
		}
		if d != nil {
			// Parent process — child has forked; parent exits.
			return
		}
		defer cntxt.Release()
	}

	log.Println("SmartType daemon starting...")

	// systray.Run must own the main goroutine (GTK/AppIndicator requirement).
	// All startup logic lives in onReady; shutdown in onExit.
	systray.Run(onReady, onExit)
}

func onReady() {
	systray.SetTitle("SmartType")
	systray.SetTooltip("SmartType — autocomplete active")
	// The AppIndicator GTK widget isn't ready at onReady entry time.
	// Delay the SetIcon call so GTK can finish initializing the widget,
	// otherwise gtk_widget_get_scale_factor asserts and the icon is skipped.
	go func() {
		time.Sleep(300 * time.Millisecond)
		systray.SetIcon(makeIcon())
	}()

	mStatus := systray.AddMenuItem("SmartType", "Typing assistant")
	mStatus.Disable()
	systray.AddSeparator()
	mReload := systray.AddMenuItem("Reload Config", "Re-read ~/.config/smarttype/config.yaml")
	systray.AddSeparator()
	mQuit := systray.AddMenuItem("Quit SmartType", "")

	service := NewService()
	if err := service.Start(); err != nil {
		log.Fatalf("Failed to start service: %v", err)
	}
	log.Println("SmartType running")

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM, syscall.SIGHUP)

	go func() {
		for {
			select {
			case sig := <-sigChan:
				switch sig {
				case syscall.SIGTERM, syscall.SIGINT:
					log.Println("Shutdown signal received")
					service.Stop()
					systray.Quit()
					return
				case syscall.SIGHUP:
					log.Println("Reloading config...")
					if err := service.Reload(); err != nil {
						log.Printf("Reload error: %v", err)
					}
				}
			case <-mReload.ClickedCh:
				log.Println("Reload requested from tray")
				if err := service.Reload(); err != nil {
					log.Printf("Reload error: %v", err)
				}
			case <-mQuit.ClickedCh:
				log.Println("Quit from tray")
				service.Stop()
				systray.Quit()
				return
			}
		}
	}()
}

func onExit() {
	log.Println("SmartType stopped")
}
