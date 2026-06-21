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
	config          *Config
	configPath      string
	engineCmd       *exec.Cmd
	engineStopped   bool // true = intentional stop, don't auto-restart
	watcher         *fsnotify.Watcher
	stopChan        chan struct{}
	wg              sync.WaitGroup
	mu              sync.RWMutex
	stats           Stats
	startTime       time.Time
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

	if err := s.startEngine(); err != nil {
		return fmt.Errorf("failed to start IBus engine: %w", err)
	}

	s.wg.Add(1)
	go s.watchConfig()

	log.Println("SmartType service started successfully")
	return nil
}

// Stop gracefully shuts down the service
func (s *Service) Stop() {
	log.Println("Stopping SmartType service...")

	s.mu.Lock()
	s.engineStopped = true
	engineCmd := s.engineCmd
	s.mu.Unlock()

	select {
	case <-s.stopChan:
	default:
		close(s.stopChan)
	}

	if engineCmd != nil && engineCmd.Process != nil {
		engineCmd.Process.Signal(os.Interrupt)
	}

	if s.watcher != nil {
		s.watcher.Close()
	}

	s.wg.Wait()
	log.Println("SmartType service stopped")
}

// Reload reloads configuration and restarts the engine
func (s *Service) Reload() error {
	log.Println("Reloading configuration...")

	if err := s.loadConfig(); err != nil {
		return fmt.Errorf("failed to reload config: %w", err)
	}

	s.mu.Lock()
	s.engineStopped = true
	oldEngine := s.engineCmd
	s.mu.Unlock()

	if oldEngine != nil && oldEngine.Process != nil {
		oldEngine.Process.Signal(os.Interrupt)
	}

	time.Sleep(400 * time.Millisecond)

	s.mu.Lock()
	s.engineStopped = false
	s.mu.Unlock()

	if err := s.startEngine(); err != nil {
		return fmt.Errorf("failed to restart IBus engine: %w", err)
	}

	s.mu.Lock()
	s.stats.LastReload = time.Now()
	s.mu.Unlock()

	log.Println("Configuration reloaded successfully")
	return nil
}

// startEngine starts (or restarts) the IBus engine process.
// A monitoring goroutine auto-restarts it on unexpected exit.
func (s *Service) startEngine() error {
	if !s.config.Enabled {
		log.Println("SmartType disabled in config, engine not started")
		return nil
	}

	path, err := findBinary("smarttype-engine")
	if err != nil {
		return err
	}

	cmd := exec.Command(path)
	cmd.Env = append(os.Environ(), "RUST_LOG=info")

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start engine: %w", err)
	}

	s.mu.Lock()
	s.engineCmd = cmd
	s.mu.Unlock()

	log.Printf("Engine started (PID %d)", cmd.Process.Pid)

	s.wg.Add(1)
	go func(c *exec.Cmd) {
		defer s.wg.Done()
		if err := c.Wait(); err != nil {
			log.Printf("Engine exited: %v", err)
		}
		s.mu.RLock()
		stopped := s.engineStopped
		s.mu.RUnlock()
		if stopped {
			return
		}
		select {
		case <-s.stopChan:
			return
		default:
			log.Printf("Engine exited unexpectedly, restarting in 2s...")
			time.Sleep(2 * time.Second)
			select {
			case <-s.stopChan:
			default:
				if err := s.startEngine(); err != nil {
					log.Printf("Engine restart failed: %v", err)
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
