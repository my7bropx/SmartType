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
	hookProcess  *os.Process
	watcher      *fsnotify.Watcher
	stopChan     chan struct{}
	wg           sync.WaitGroup
	mu           sync.RWMutex
	stats        Stats
	startTime    time.Time
}

// Config represents the SmartType configuration
type Config struct {
	Enabled          bool                       `yaml:"enabled"`
	SmartPunctuation bool                       `yaml:"smart_punctuation"`
	Autocorrect      bool                       `yaml:"autocorrect"`
	MinWordLength    int                        `yaml:"min_word_length"`
	Applications     map[string]AppConfig       `yaml:"applications"`
	CustomTypos      map[string]string          `yaml:"custom_typos"`
	Hotkey           string                     `yaml:"hotkey"`
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

// Start initializes and starts the service
func (s *Service) Start() error {
	log.Println("Starting SmartType service...")

	// Load configuration
	if err := s.loadConfig(); err != nil {
		return fmt.Errorf("failed to load config: %w", err)
	}

	// Setup file watcher for config changes
	if err := s.setupWatcher(); err != nil {
		return fmt.Errorf("failed to setup config watcher: %w", err)
	}

	// Start the input hook process
	if err := s.startHook(); err != nil {
		return fmt.Errorf("failed to start input hook: %w", err)
	}

	// Start background tasks
	s.wg.Add(1)
	go s.watchConfig()

	log.Println("SmartType service started successfully")
	return nil
}

// Stop gracefully shuts down the service
func (s *Service) Stop() {
	log.Println("Stopping SmartType service...")

	close(s.stopChan)

	// Stop the hook process
	if s.hookProcess != nil {
		s.hookProcess.Signal(os.Interrupt)
		s.hookProcess.Wait()
	}

	// Stop file watcher
	if s.watcher != nil {
		s.watcher.Close()
	}

	s.wg.Wait()
	log.Println("SmartType service stopped")
}

// Reload reloads the configuration
func (s *Service) Reload() error {
	log.Println("Reloading configuration...")

	if err := s.loadConfig(); err != nil {
		return fmt.Errorf("failed to reload config: %w", err)
	}

	// Restart hook process with new config
	if s.hookProcess != nil {
		s.hookProcess.Signal(os.Interrupt)
		s.hookProcess.Wait()
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

// loadConfig loads configuration from file
func (s *Service) loadConfig() error {
	data, err := os.ReadFile(s.configPath)
	if err != nil {
		if os.IsNotExist(err) {
			// Create default config
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

	// Ensure config directory exists
	configDir := filepath.Dir(s.configPath)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return err
	}

	return os.WriteFile(s.configPath, data, 0644)
}

// defaultConfig returns default configuration
func (s *Service) defaultConfig() *Config {
	trueVal := true
	falseVal := false

	return &Config{
		Enabled:          true,
		SmartPunctuation: true,
		Autocorrect:      true,
		MinWordLength:    2,
		Applications: map[string]AppConfig{
			"firefox": {
				Enabled:     true,
				SmartQuotes: &trueVal,
				Autocorrect: &trueVal,
			},
			"qterminal": {
				Enabled:     true,
				SmartQuotes: &falseVal,
				Autocorrect: &trueVal,
			},
			"kitty": {
				Enabled:     true,
				SmartQuotes: &falseVal,
				Autocorrect: &trueVal,
			},
		},
		CustomTypos: map[string]string{
			"hte":     "the",
			"becuase": "because",
		},
		Hotkey: "Super+Shift+A",
	}
}

// setupWatcher sets up file system watcher for config changes
func (s *Service) setupWatcher() error {
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return err
	}

	s.watcher = watcher

	// Watch config directory
	configDir := filepath.Dir(s.configPath)
	if err := watcher.Add(configDir); err != nil {
		return err
	}

	return nil
}

// watchConfig watches for configuration file changes
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

// startHook starts the input hook process
func (s *Service) startHook() error {
	if !s.config.Enabled {
		log.Println("SmartType is disabled in config, not starting hook")
		return nil
	}

	// Find hook binary
	hookPath := "/usr/local/bin/smarttype-hook"
	if _, err := os.Stat(hookPath); os.IsNotExist(err) {
		hookPath = "./target/release/smarttype-hook"
	}

	cmd := exec.Command(hookPath)
	cmd.Env = append(os.Environ(), "RUST_LOG=info")

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start hook process: %w", err)
	}

	s.hookProcess = cmd.Process
	log.Printf("Input hook started (PID: %d)", s.hookProcess.Pid)

	return nil
}

// GetStats returns current statistics
func (s *Service) GetStats() Stats {
	s.mu.RLock()
	defer s.mu.RUnlock()

	stats := s.stats
	stats.Uptime = time.Since(s.startTime)
	return stats
}
