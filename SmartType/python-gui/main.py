#!/usr/bin/env python3
"""
SmartType Configuration GUI
Professional Qt-based interface for managing SmartType settings
"""

import sys
import os
import yaml
import subprocess
from pathlib import Path
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QCheckBox, QPushButton, QTabWidget, QTableWidget,
    QTableWidgetItem, QLineEdit, QSpinBox, QGroupBox, QMessageBox,
    QHeaderView, QSystemTrayIcon, QMenu, QAction
)
from PyQt5.QtCore import Qt, QTimer
from PyQt5.QtGui import QIcon, QFont


class SmartTypeConfig:
    """Manages SmartType configuration"""

    def __init__(self):
        self.config_dir = Path.home() / '.config' / 'smarttype'
        self.config_file = self.config_dir / 'config.yaml'
        self.config = self.load_config()

    def load_config(self):
        """Load configuration from file"""
        if self.config_file.exists():
            with open(self.config_file, 'r') as f:
                return yaml.safe_load(f)
        return self.default_config()

    def save_config(self):
        """Save configuration to file"""
        self.config_dir.mkdir(parents=True, exist_ok=True)
        with open(self.config_file, 'w') as f:
            yaml.dump(self.config, f, default_flow_style=False)

    def default_config(self):
        """Return default configuration"""
        return {
            'enabled': True,
            'smart_punctuation': True,
            'autocorrect': True,
            'min_word_length': 2,
            'applications': {
                'firefox': {'enabled': True, 'smart_quotes': True, 'autocorrect': True},
                'qterminal': {'enabled': True, 'smart_quotes': False, 'autocorrect': True},
                'kitty': {'enabled': True, 'smart_quotes': False, 'autocorrect': True},
            },
            'custom_typos': {
                'hte': 'the',
                'becuase': 'because',
            },
            'hotkey': 'Super+Shift+A'
        }


class MainWindow(QMainWindow):
    """Main application window"""

    def __init__(self):
        super().__init__()
        self.config_manager = SmartTypeConfig()
        self.init_ui()
        self.load_settings()
        self.setup_tray_icon()

    def init_ui(self):
        """Initialize user interface"""
        self.setWindowTitle('SmartType Configuration')
        self.setGeometry(100, 100, 800, 600)

        # Create central widget
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        # Main layout
        layout = QVBoxLayout()
        central_widget.setLayout(layout)

        # Header
        header = self.create_header()
        layout.addWidget(header)

        # Tab widget
        tabs = QTabWidget()
        tabs.addTab(self.create_general_tab(), "General")
        tabs.addTab(self.create_applications_tab(), "Applications")
        tabs.addTab(self.create_custom_typos_tab(), "Custom Corrections")
        tabs.addTab(self.create_statistics_tab(), "Statistics")
        layout.addWidget(tabs)

        # Footer buttons
        footer = self.create_footer()
        layout.addWidget(footer)

    def create_header(self):
        """Create header section"""
        header = QGroupBox()
        layout = QHBoxLayout()

        # Title and status
        title_layout = QVBoxLayout()
        title = QLabel("SmartType")
        title.setFont(QFont("Arial", 18, QFont.Bold))
        subtitle = QLabel("System-wide autocorrect and smart punctuation for Linux")
        subtitle.setStyleSheet("color: gray;")

        title_layout.addWidget(title)
        title_layout.addWidget(subtitle)
        layout.addLayout(title_layout)

        layout.addStretch()

        # Status indicator
        self.status_label = QLabel("Status: ")
        self.status_indicator = QLabel("●")
        self.status_indicator.setStyleSheet("color: green; font-size: 20px;")

        status_layout = QHBoxLayout()
        status_layout.addWidget(self.status_label)
        status_layout.addWidget(self.status_indicator)
        layout.addLayout(status_layout)

        header.setLayout(layout)
        return header

    def create_general_tab(self):
        """Create general settings tab"""
        widget = QWidget()
        layout = QVBoxLayout()

        # Enable/disable SmartType
        self.enabled_checkbox = QCheckBox("Enable SmartType")
        self.enabled_checkbox.setToolTip("Master switch for all autocorrect features")
        layout.addWidget(self.enabled_checkbox)

        # Features group
        features_group = QGroupBox("Features")
        features_layout = QVBoxLayout()

        self.autocorrect_checkbox = QCheckBox("Enable Autocorrect")
        self.autocorrect_checkbox.setToolTip("Automatically fix common typos")
        features_layout.addWidget(self.autocorrect_checkbox)

        self.smart_punctuation_checkbox = QCheckBox("Enable Smart Punctuation")
        self.smart_punctuation_checkbox.setToolTip("Convert straight quotes to curly quotes, etc.")
        features_layout.addWidget(self.smart_punctuation_checkbox)

        features_group.setLayout(features_layout)
        layout.addWidget(features_group)

        # Settings group
        settings_group = QGroupBox("Settings")
        settings_layout = QVBoxLayout()

        # Minimum word length
        min_word_layout = QHBoxLayout()
        min_word_label = QLabel("Minimum word length:")
        self.min_word_spinbox = QSpinBox()
        self.min_word_spinbox.setMinimum(1)
        self.min_word_spinbox.setMaximum(10)
        self.min_word_spinbox.setToolTip("Minimum length of words to autocorrect")
        min_word_layout.addWidget(min_word_label)
        min_word_layout.addWidget(self.min_word_spinbox)
        min_word_layout.addStretch()
        settings_layout.addLayout(min_word_layout)

        # Hotkey
        hotkey_layout = QHBoxLayout()
        hotkey_label = QLabel("Toggle hotkey:")
        self.hotkey_input = QLineEdit()
        self.hotkey_input.setPlaceholderText("e.g., Super+Shift+A")
        hotkey_layout.addWidget(hotkey_label)
        hotkey_layout.addWidget(self.hotkey_input)
        settings_layout.addLayout(hotkey_layout)

        settings_group.setLayout(settings_layout)
        layout.addWidget(settings_group)

        layout.addStretch()
        widget.setLayout(layout)
        return widget

    def create_applications_tab(self):
        """Create per-application settings tab"""
        widget = QWidget()
        layout = QVBoxLayout()

        label = QLabel("Configure SmartType behavior for specific applications:")
        layout.addWidget(label)

        # Application table
        self.app_table = QTableWidget()
        self.app_table.setColumnCount(4)
        self.app_table.setHorizontalHeaderLabels(['Application', 'Enabled', 'Smart Quotes', 'Autocorrect'])
        self.app_table.horizontalHeader().setSectionResizeMode(0, QHeaderView.Stretch)
        layout.addWidget(self.app_table)

        # Add/Remove buttons
        button_layout = QHBoxLayout()
        add_app_btn = QPushButton("Add Application")
        add_app_btn.clicked.connect(self.add_application)
        remove_app_btn = QPushButton("Remove Selected")
        remove_app_btn.clicked.connect(self.remove_application)
        button_layout.addWidget(add_app_btn)
        button_layout.addWidget(remove_app_btn)
        button_layout.addStretch()
        layout.addLayout(button_layout)

        widget.setLayout(layout)
        return widget

    def create_custom_typos_tab(self):
        """Create custom typo corrections tab"""
        widget = QWidget()
        layout = QVBoxLayout()

        label = QLabel("Add your own typo corrections:")
        layout.addWidget(label)

        # Custom typos table
        self.typo_table = QTableWidget()
        self.typo_table.setColumnCount(2)
        self.typo_table.setHorizontalHeaderLabels(['Typo', 'Correction'])
        self.typo_table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        layout.addWidget(self.typo_table)

        # Add/Remove buttons
        button_layout = QHBoxLayout()
        add_typo_btn = QPushButton("Add Correction")
        add_typo_btn.clicked.connect(self.add_typo)
        remove_typo_btn = QPushButton("Remove Selected")
        remove_typo_btn.clicked.connect(self.remove_typo)
        button_layout.addWidget(add_typo_btn)
        button_layout.addWidget(remove_typo_btn)
        button_layout.addStretch()
        layout.addLayout(button_layout)

        widget.setLayout(layout)
        return widget

    def create_statistics_tab(self):
        """Create statistics tab"""
        widget = QWidget()
        layout = QVBoxLayout()

        stats_group = QGroupBox("Statistics")
        stats_layout = QVBoxLayout()

        self.total_corrections_label = QLabel("Total corrections: 0")
        self.session_corrections_label = QLabel("Session corrections: 0")
        self.uptime_label = QLabel("Uptime: N/A")
        self.dictionary_size_label = QLabel("Dictionary size: 2000+ typos")

        stats_layout.addWidget(self.total_corrections_label)
        stats_layout.addWidget(self.session_corrections_label)
        stats_layout.addWidget(self.uptime_label)
        stats_layout.addWidget(self.dictionary_size_label)

        stats_group.setLayout(stats_layout)
        layout.addWidget(stats_group)

        # Refresh button
        refresh_btn = QPushButton("Refresh Statistics")
        refresh_btn.clicked.connect(self.refresh_statistics)
        layout.addWidget(refresh_btn)

        layout.addStretch()
        widget.setLayout(layout)
        return widget

    def create_footer(self):
        """Create footer with action buttons"""
        footer = QWidget()
        layout = QHBoxLayout()

        # Service control buttons
        self.start_btn = QPushButton("Start Service")
        self.start_btn.clicked.connect(self.start_service)

        self.stop_btn = QPushButton("Stop Service")
        self.stop_btn.clicked.connect(self.stop_service)

        self.restart_btn = QPushButton("Restart Service")
        self.restart_btn.clicked.connect(self.restart_service)

        layout.addWidget(self.start_btn)
        layout.addWidget(self.stop_btn)
        layout.addWidget(self.restart_btn)
        layout.addStretch()

        # Save/Cancel buttons
        save_btn = QPushButton("Save Configuration")
        save_btn.clicked.connect(self.save_configuration)
        save_btn.setStyleSheet("QPushButton { background-color: #4CAF50; color: white; padding: 5px 15px; }")

        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.load_settings)

        layout.addWidget(cancel_btn)
        layout.addWidget(save_btn)

        footer.setLayout(layout)
        return footer

    def setup_tray_icon(self):
        """Setup system tray icon"""
        self.tray_icon = QSystemTrayIcon(self)

        # Create tray menu
        tray_menu = QMenu()

        show_action = QAction("Show Configuration", self)
        show_action.triggered.connect(self.show)
        tray_menu.addAction(show_action)

        tray_menu.addSeparator()

        toggle_action = QAction("Enable/Disable", self)
        toggle_action.triggered.connect(self.toggle_smarttype)
        tray_menu.addAction(toggle_action)

        tray_menu.addSeparator()

        quit_action = QAction("Quit", self)
        quit_action.triggered.connect(QApplication.quit)
        tray_menu.addAction(quit_action)

        self.tray_icon.setContextMenu(tray_menu)
        self.tray_icon.activated.connect(self.tray_icon_activated)

        # Set icon (would need an actual icon file)
        # self.tray_icon.setIcon(QIcon('smarttype.png'))

        self.tray_icon.show()

    def tray_icon_activated(self, reason):
        """Handle tray icon activation"""
        if reason == QSystemTrayIcon.DoubleClick:
            self.show()

    def load_settings(self):
        """Load settings from configuration"""
        config = self.config_manager.config

        # General settings
        self.enabled_checkbox.setChecked(config.get('enabled', True))
        self.autocorrect_checkbox.setChecked(config.get('autocorrect', True))
        self.smart_punctuation_checkbox.setChecked(config.get('smart_punctuation', True))
        self.min_word_spinbox.setValue(config.get('min_word_length', 2))
        self.hotkey_input.setText(config.get('hotkey', 'Super+Shift+A'))

        # Applications
        self.load_applications_table()

        # Custom typos
        self.load_typos_table()

    def load_applications_table(self):
        """Load applications into table"""
        apps = self.config_manager.config.get('applications', {})
        self.app_table.setRowCount(len(apps))

        for row, (app_name, app_config) in enumerate(apps.items()):
            # Application name
            self.app_table.setItem(row, 0, QTableWidgetItem(app_name))

            # Enabled checkbox
            enabled_checkbox = QCheckBox()
            enabled_checkbox.setChecked(app_config.get('enabled', True))
            self.app_table.setCellWidget(row, 1, enabled_checkbox)

            # Smart quotes checkbox
            sq_checkbox = QCheckBox()
            sq_checkbox.setChecked(app_config.get('smart_quotes', True))
            self.app_table.setCellWidget(row, 2, sq_checkbox)

            # Autocorrect checkbox
            ac_checkbox = QCheckBox()
            ac_checkbox.setChecked(app_config.get('autocorrect', True))
            self.app_table.setCellWidget(row, 3, ac_checkbox)

    def load_typos_table(self):
        """Load custom typos into table"""
        typos = self.config_manager.config.get('custom_typos', {})
        self.typo_table.setRowCount(len(typos))

        for row, (typo, correction) in enumerate(typos.items()):
            self.typo_table.setItem(row, 0, QTableWidgetItem(typo))
            self.typo_table.setItem(row, 1, QTableWidgetItem(correction))

    def save_configuration(self):
        """Save current configuration"""
        config = self.config_manager.config

        # Update general settings
        config['enabled'] = self.enabled_checkbox.isChecked()
        config['autocorrect'] = self.autocorrect_checkbox.isChecked()
        config['smart_punctuation'] = self.smart_punctuation_checkbox.isChecked()
        config['min_word_length'] = self.min_word_spinbox.value()
        config['hotkey'] = self.hotkey_input.text()

        # Update applications
        apps = {}
        for row in range(self.app_table.rowCount()):
            app_name = self.app_table.item(row, 0).text()
            enabled = self.app_table.cellWidget(row, 1).isChecked()
            smart_quotes = self.app_table.cellWidget(row, 2).isChecked()
            autocorrect = self.app_table.cellWidget(row, 3).isChecked()

            apps[app_name] = {
                'enabled': enabled,
                'smart_quotes': smart_quotes,
                'autocorrect': autocorrect
            }
        config['applications'] = apps

        # Update custom typos
        typos = {}
        for row in range(self.typo_table.rowCount()):
            typo = self.typo_table.item(row, 0).text()
            correction = self.typo_table.item(row, 1).text()
            typos[typo] = correction
        config['custom_typos'] = typos

        # Save to file
        self.config_manager.save_config()

        QMessageBox.information(self, "Configuration Saved", "Configuration has been saved successfully.")

        # Reload service
        self.restart_service()

    def start_service(self):
        """Start SmartType service"""
        try:
            subprocess.run(['systemctl', '--user', 'start', 'smarttype'], check=True)
            QMessageBox.information(self, "Service Started", "SmartType service has been started.")
            self.update_status()
        except subprocess.CalledProcessError as e:
            QMessageBox.warning(self, "Error", f"Failed to start service: {e}")

    def stop_service(self):
        """Stop SmartType service"""
        try:
            subprocess.run(['systemctl', '--user', 'stop', 'smarttype'], check=True)
            QMessageBox.information(self, "Service Stopped", "SmartType service has been stopped.")
            self.update_status()
        except subprocess.CalledProcessError as e:
            QMessageBox.warning(self, "Error", f"Failed to stop service: {e}")

    def restart_service(self):
        """Restart SmartType service"""
        try:
            subprocess.run(['systemctl', '--user', 'restart', 'smarttype'], check=True)
            QMessageBox.information(self, "Service Restarted", "SmartType service has been restarted.")
            self.update_status()
        except subprocess.CalledProcessError as e:
            QMessageBox.warning(self, "Error", f"Failed to restart service: {e}")

    def toggle_smarttype(self):
        """Toggle SmartType on/off"""
        self.enabled_checkbox.setChecked(not self.enabled_checkbox.isChecked())
        self.save_configuration()

    def add_application(self):
        """Add new application"""
        from PyQt5.QtWidgets import QInputDialog

        app_name, ok = QInputDialog.getText(self, "Add Application", "Application name:")
        if ok and app_name:
            row = self.app_table.rowCount()
            self.app_table.insertRow(row)
            self.app_table.setItem(row, 0, QTableWidgetItem(app_name))

            for col in range(1, 4):
                checkbox = QCheckBox()
                checkbox.setChecked(True)
                self.app_table.setCellWidget(row, col, checkbox)

    def remove_application(self):
        """Remove selected application"""
        current_row = self.app_table.currentRow()
        if current_row >= 0:
            self.app_table.removeRow(current_row)

    def add_typo(self):
        """Add new typo correction"""
        from PyQt5.QtWidgets import QInputDialog

        typo, ok1 = QInputDialog.getText(self, "Add Typo", "Typo:")
        if ok1 and typo:
            correction, ok2 = QInputDialog.getText(self, "Add Correction", "Correction:")
            if ok2 and correction:
                row = self.typo_table.rowCount()
                self.typo_table.insertRow(row)
                self.typo_table.setItem(row, 0, QTableWidgetItem(typo))
                self.typo_table.setItem(row, 1, QTableWidgetItem(correction))

    def remove_typo(self):
        """Remove selected typo"""
        current_row = self.typo_table.currentRow()
        if current_row >= 0:
            self.typo_table.removeRow(current_row)

    def refresh_statistics(self):
        """Refresh statistics display"""
        # In a real implementation, this would query the daemon
        QMessageBox.information(self, "Statistics", "Statistics updated (feature in development)")

    def update_status(self):
        """Update service status indicator"""
        try:
            result = subprocess.run(
                ['systemctl', '--user', 'is-active', 'smarttype'],
                capture_output=True,
                text=True
            )
            is_active = result.returncode == 0

            if is_active:
                self.status_indicator.setStyleSheet("color: green; font-size: 20px;")
                self.status_label.setText("Status: Active")
            else:
                self.status_indicator.setStyleSheet("color: red; font-size: 20px;")
                self.status_label.setText("Status: Inactive")
        except:
            self.status_indicator.setStyleSheet("color: gray; font-size: 20px;")
            self.status_label.setText("Status: Unknown")


def main():
    """Main entry point"""
    app = QApplication(sys.argv)
    app.setApplicationName("SmartType")

    window = MainWindow()
    window.show()

    sys.exit(app.exec_())


if __name__ == '__main__':
    main()
