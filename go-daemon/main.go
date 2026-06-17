package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/sevlyar/go-daemon"
)

var (
	signal_flag = flag.String("s", "", "send signal to daemon (stop, reload)")
	daemonize   = flag.Bool("d", false, "run as daemon")
)

func main() {
	flag.Parse()

	// Setup daemon context
	cntxt := &daemon.Context{
		PidFileName: "/tmp/smarttype.pid",
		PidFilePerm: 0644,
		LogFileName: "/tmp/smarttype.log",
		LogFilePerm: 0640,
		WorkDir:     "/",
		Umask:       027,
	}

	// Handle signals
	if len(*signal_flag) > 0 {
		daemon_process, err := cntxt.Search()
		if err != nil {
			log.Fatalf("Unable to send signal to daemon: %s", err.Error())
		}

		switch *signal_flag {
		case "stop":
			daemon_process.Signal(syscall.SIGTERM)
			fmt.Println("SmartType daemon stopped")
		case "reload":
			daemon_process.Signal(syscall.SIGHUP)
			fmt.Println("SmartType daemon reloaded")
		default:
			fmt.Println("Unknown signal:", *signal_flag)
		}
		return
	}

	// Run as daemon if requested
	if *daemonize {
		d, err := cntxt.Reborn()
		if err != nil {
			log.Fatal("Unable to run as daemon: ", err)
		}
		if d != nil {
			return
		}
		defer cntxt.Release()
	}

	log.Println("SmartType daemon starting...")

	// Run service
	service := NewService()
	if err := service.Start(); err != nil {
		log.Fatal("Failed to start service: ", err)
	}

	// Setup signal handling
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM, syscall.SIGHUP)

	// Main loop
	for {
		sig := <-sigChan
		switch sig {
		case syscall.SIGTERM, syscall.SIGINT:
			log.Println("Received termination signal, shutting down...")
			service.Stop()
			return
		case syscall.SIGHUP:
			log.Println("Received reload signal, reloading configuration...")
			if err := service.Reload(); err != nil {
				log.Printf("Error reloading: %v", err)
			}
		}
	}
}
