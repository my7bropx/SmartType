#!/bin/bash
# Fix SmartType systemd service

echo "Fixing SmartType systemd service..."

# Update the service file to remove -d flag
sudo sed -i 's|ExecStart=/usr/local/bin/smarttype-daemon -d|ExecStart=/usr/local/bin/smarttype-daemon|' /usr/lib/systemd/user/smarttype.service

echo "Reloading systemd configuration..."
systemctl --user daemon-reload

echo "Starting SmartType service..."
systemctl --user start smarttype

echo ""
echo "Checking service status..."
systemctl --user status smarttype --no-pager

echo ""
echo "✓ SmartType should now be running!"
echo "Try typing 'teh' in any application - it should autocorrect to 'the'"
