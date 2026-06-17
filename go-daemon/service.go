package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"time"

	"github.com/fsnotify/fsnotify"
	"gopkg.in/yaml.v3"
)

// Service manages the SmartType daemon
type Service struct {
	config       *Config
	configPath   string
	hookCmd      *exec.Cmd
	popupCmd     *exec.Cmd
	hookStopped  bool // true = intentional stop, don't auto-restart
	popupStopped bool
	watcher      *fsnotify.Watcher
	stopChan     chan struct{}
	wg           sync.WaitGroup
	mu           sync.RWMutex
	stats        Stats
	startTime    time.Time
}

// Config represents the SmartType configuration
type Config struct {
	Enabled          bool                 `yaml:"enabled"`
	SmartPunctuation bool                 `yaml:"smart_punctuation"`
	Autocorrect      bool                 `yaml:"autocorrect"`
	MinWordLength    int                  `yaml:"min_word_length"`
	Applications     map[string]AppConfig `yaml:"applications"`
	CustomTypos      map[string]string    `yaml:"custom_typos"`
	Hotkey           string               `yaml:"hotkey"`
}

// AppConfig represents per-application configuration
type AppConfig struct {
	Enabled     bool  `yaml:"enabled"`
	SmartQuotes *bool `yaml:"smart_quotes,omitempty"`
	Autocorrect *bool `yaml:"autocorrect,omitempty"`
}

// Stats tracks daemon statistics
type Stats struct {
	TotalCorrections   uint64
	SessionCorrections uint64
	Uptime             time.Duration
	LastReload         time.Time
}

// NewService creates a new service instance
func NewService() *Service {
	homeDir, _ := os.UserHomeDir()
	configPath := filepath.Join(homeDir, ".config", "smarttype", "config.yaml")

	return &Service{
		configPath: configPath,
		stopChan:   make(chan struct{}),
		startTime:  time.Now(),
	}
}

// findBinary locates a named binary: same dir as the daemon executable,
// then ../rust-core/target/release/ (dev layout), then /usr/local/bin/.
func findBinary(name string) (string, error) {
	if exe, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exe)
		candidates := []string{
			filepath.Join(exeDir, name),
			filepath.Join(exeDir, "..", "rust-core", "target", "release", name),
			filepath.Join(exeDir, "..", "SmartType", "rust-core", "target", "release", name),
		}
		for _, p := range candidates {
			if _, err := os.Stat(p); err == nil {
				return filepath.Clean(p), nil
			}
		}
	}
	sys := "/usr/local/bin/" + name
	if _, err := os.Stat(sys); err == nil {
		return sys, nil
	}
	return "", fmt.Errorf("binary %q not found (tried exe-dir, ../rust-core/target/release, /usr/local/bin)", name)
}

// Start initializes and starts the service
func (s *Service) Start() error {
	log.Println("Starting SmartType service...")

	if err := s.loadConfig(); err != nil {
		return fmt.Errorf("failed to load config: %w", err)
	}

	if err := s.setupWatcher(); err != nil {
		return fmt.Errorf("failed to setup config watcher: %w", err)
	}

	if err := s.startPopup(); err != nil {
		log.Printf("Warning: could not start popup: %v", err)
	}

	if err := s.startHook(); err != nil {
		return fmt.Errorf("failed to start input hook: %w", err)
	}

	s.wg.Add(1)
	go s.watchConfig()

	log.Println("SmartType service started successfully")
	return nil
}

// Stop gracefully shuts down the service
func (s *Service) Stop() {
	log.Println("Stopping SmartType service...")

	// Mark children as intentionally stopped before closing stopChan,
	// so that monitoring goroutines don't race to restart them.
	s.mu.Lock()
	s.hookStopped = true
	s.popupStopped = true
	hookCmd := s.hookCmd
	popupCmd := s.popupCmd
	s.mu.Unlock()

	select {
	case <-s.stopChan:
	default:
		close(s.stopChan)
	}

	if hookCmd != nil && hookCmd.Process != nil {
		hookCmd.Process.Signal(os.Interrupt)
	}
	if popupCmd != nil && popupCmd.Process != nil {
		popupCmd.Process.Signal(os.Interrupt)
	}

	if s.watcher != nil {
		s.watcher.Close()
	}

	s.wg.Wait()
	log.Println("SmartType service stopped")
}

// Reload reloads configuration and restarts child processes
func (s *Service) Reload() error {
	log.Println("Reloading configuration...")

	if err := s.loadConfig(); err != nil {
		return fmt.Errorf("failed to reload config: %w", err)
	}

	// Mark as intentional stop so monitoring goroutines don't restart old instances
	s.mu.Lock()
	s.hookStopped = true
	s.popupStopped = true
	oldHook := s.hookCmd
	oldPopup := s.popupCmd
	s.mu.Unlock()

	if oldPopup != nil && oldPopup.Process != nil {
		oldPopup.Process.Signal(os.Interrupt)
	}
	if oldHook != nil && oldHook.Process != nil {
		oldHook.Process.Signal(os.Interrupt)
	}

	// Give old processes time to exit cleanly
	time.Sleep(400 * time.Millisecond)

	// Re-enable auto-restart before launching new instances
	s.mu.Lock()
	s.hookStopped = false
	s.popupStopped = false
	s.mu.Unlock()

	if err := s.startPopup(); err != nil {
		log.Printf("Warning: could not restart popup: %v", err)
	}
	if err := s.startHook(); err != nil {
		return fmt.Errorf("failed to restart hook: %w", err)
	}

	s.mu.Lock()
	s.stats.LastReload = time.Now()
	s.mu.Unlock()

	log.Println("Configuration reloaded successfully")
	return nil
}

// startPopup starts (or restarts) the X11 suggestion overlay.
// It launches a monitoring goroutine that auto-restarts on unexpected exit.
func (s *Service) startPopup() error {
	path, err := findBinary("smarttype-popup")
	if err != nil {
		return err
	}

	env := os.Environ()
	// Ensure DISPLAY is set for X11 popup
	hasDisplay := false
	for _, e := range env {
		if len(e) >= 8 && e[:8] == "DISPLAY=" {
			hasDisplay = true
			break
		}
	}
	if !hasDisplay {
		env = append(env, "DISPLAY=:0")
	}
	env = append(env, "RUST_LOG=warn")

	cmd := exec.Command(path)
	cmd.Env = env

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start popup: %w", err)
	}

	s.mu.Lock()
	s.popupCmd = cmd
	s.mu.Unlock()

	log.Printf("Popup started (PID %d)", cmd.Process.Pid)

	s.wg.Add(1)
	go func(c *exec.Cmd) {
		defer s.wg.Done()
		if err := c.Wait(); err != nil {
			log.Printf("Popup exited: %v", err)
		}
		s.mu.RLock()
		stopped := s.popupStopped
		s.mu.RUnlock()
		if stopped {
			return
		}
		select {
		case <-s.stopChan:
			return
		default:
			log.Printf("Popup exited unexpectedly, restarting in 3s...")
			time.Sleep(3 * time.Second)
			select {
			case <-s.stopChan:
			default:
				if err := s.startPopup(); err != nil {
					log.Printf("Popup restart failed: %v", err)
				}
			}
		}
	}(cmd)

	return nil
}

// startHook starts (or restarts) the input hook process.
// It launches a monitoring goroutine that auto-restarts on unexpected exit.
func (s *Service) startHook() error {
	if !s.config.Enabled {
		log.Println("SmartType disabled in config, hook not started")
		return nil
	}

	path, err := findBinary("smarttype-hook")
	if err != nil {
		return err
	}

	cmd := exec.Command(path)
	cmd.Env = append(os.Environ(), "RUST_LOG=info")

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start hook: %w", err)
	}

	s.mu.Lock()
	s.hookCmd = cmd
	s.mu.Unlock()

	log.Printf("Hook started (PID %d)", cmd.Process.Pid)

	s.wg.Add(1)
	go func(c *exec.Cmd) {
		defer s.wg.Done()
		if err := c.Wait(); err != nil {
			log.Printf("Hook exited: %v", err)
		}
		s.mu.RLock()
		stopped := s.hookStopped
		s.mu.RUnlock()
		if stopped {
			return
		}
		select {
		case <-s.stopChan:
			return
		default:
			log.Printf("Hook exited unexpectedly, restarting in 2s...")
			time.Sleep(2 * time.Second)
			select {
			case <-s.stopChan:
			default:
				if err := s.startHook(); err != nil {
					log.Printf("Hook restart failed: %v", err)
				}
			}
		}
	}(cmd)

	return nil
}

// loadConfig loads configuration from file
func (s *Service) loadConfig() error {
	data, err := os.ReadFile(s.configPath)
	if err != nil {
		if os.IsNotExist(err) {
			s.config = s.defaultConfig()
			return s.saveConfig()
		}
		return err
	}

	config := &Config{}
	if err := yaml.Unmarshal(data, config); err != nil {
		return err
	}

	s.mu.Lock()
	s.config = config
	s.mu.Unlock()

	return nil
}

// saveConfig saves configuration to file
func (s *Service) saveConfig() error {
	data, err := yaml.Marshal(s.config)
	if err != nil {
		return err
	}

	configDir := filepath.Dir(s.configPath)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return err
	}

	return os.WriteFile(s.configPath, data, 0644)
}

// defaultConfig returns the default configuration
func (s *Service) defaultConfig() *Config {
	trueVal := true
	falseVal := false

	return &Config{
		Enabled:          true,
		SmartPunctuation: true,
		Autocorrect:      true,
		MinWordLength:    2,
		Applications: map[string]AppConfig{
			"firefox":   {Enabled: true, SmartQuotes: &trueVal, Autocorrect: &trueVal},
			"qterminal": {Enabled: true, SmartQuotes: &falseVal, Autocorrect: &trueVal},
			"kitty":     {Enabled: true, SmartQuotes: &falseVal, Autocorrect: &trueVal},
		},
		CustomTypos: map[string]string{
			"hte":     "the",
			"becuase": "because",
		},
		Hotkey: "Super+Shift+A",
	}
}

// setupWatcher sets up a filesystem watcher for config changes
func (s *Service) setupWatcher() error {
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return err
	}

	s.watcher = watcher

	configDir := filepath.Dir(s.configPath)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return err
	}
	return watcher.Add(configDir)
}

// watchConfig watches for configuration file changes and reloads
func (s *Service) watchConfig() {
	defer s.wg.Done()

	for {
		select {
		case event := <-s.watcher.Events:
			if event.Name == s.configPath && event.Op&fsnotify.Write == fsnotify.Write {
				log.Println("Config file changed, reloading...")
				if err := s.Reload(); err != nil {
					log.Printf("Error reloading config: %v", err)
				}
			}
		case err := <-s.watcher.Errors:
			log.Printf("Watcher error: %v", err)
		case <-s.stopChan:
			return
		}
	}
}

// GetStats returns current statistics
func (s *Service) GetStats() Stats {
	s.mu.RLock()
	defer s.mu.RUnlock()

	stats := s.stats
	stats.Uptime = time.Since(s.startTime)
	return stats
}
