# C2R2 Team Client

A graphical user interface (GUI) for operators to connect to C2R2 Command & Control servers via SSH.

## Architecture

Similar to Havoc C2's architecture:
- **C2R2 Server**: Runs on red team infrastructure
- **Team Client**: Operators connect from their machines via SSH
- **GUI Interface**: Visual display of connected agents, command execution, etc.

```
┌─────────────────────┐                    ┌─────────────────────┐
│   Operator Machine  │     SSH Tunnel     │  Red Team Server    │
│  ┌───────────────┐  │                    │  ┌───────────────┐  │
│  │ Team Client   │──┼────────────────────┼──│  C2R2 Server  │  │
│  │   (GUI)       │  │                    │  │               │  │
│  └───────────────┘  │                    │  └───────────────┘  │
└─────────────────────┘                    │         │          │
                                           │         ▼          │
                                           │  ┌───────────────┐  │
                                           │  │   Agents      │  │
                                           │  │  (Windows)    │  │
                                           │  └───────────────┘  │
                                           └─────────────────────┘
```

## Features

- **SSH Connection**: Secure tunnel to the C2R2 server
- **Cross-Platform**: Works on Windows and Linux (tkinter-based)
- **Dark Theme**: Modern dark interface
- **Agent Management**: View connected agents in real-time
- **Command Execution**: Send commands to selected agents
- **Command History**: Navigate with arrow keys
- **Quick Actions**: One-click common commands
- **Help System**: Built-in command reference

## Requirements

- Python 3.8+
- paramiko (SSH library)
- tkinter (usually included with Python)

## Installation

### Windows

```bash
# Install Python dependencies
pip install paramiko

# Run the client
python c2r2_team_client.py
```

### Linux

```bash
# Install tkinter if not already installed
# Ubuntu/Debian:
sudo apt-get install python3-tk

# Fedora:
sudo dnf install python3-tkinter

# Install Python dependencies
pip install paramiko

# Run the client
python3 c2r2_team_client.py
```

## Usage

### 1. Start the C2R2 Server (on red team infrastructure)

```bash
# On the server
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

### 2. Launch the Team Client

```bash
python c2r2_team_client.py
```

### 3. Connect via SSH

Enter the following details in the login screen:
- **SSH Host**: IP address of the red team server
- **SSH Port**: SSH port (default 22)
- **Username**: Your SSH username
- **Password/Key**: SSH password or path to private key
- **C2 Server Port**: Port where C2R2 server is running (default 4444)
- **C2 Binary Path**: (Optional) Path to c2r2-server binary to auto-start

### 4. Interact with Agents

Once connected, you'll see:
- **Left Panel**: List of connected agents
- **Right Panel**: Console output and command input

Select an agent by clicking on it, then use commands like:
- `/cmd whoami` - Execute command on selected agent
- `/harvest` - Harvest credentials
- `/download C:\file.txt` - Download file
- `/help` - Show all commands

## Available Commands

```
📋 Client Management:
   /list                  - List all connected clients
   /select <id>           - Select a client by ID
   /deselect              - Deselect current client
   /info <id>             - Show detailed client info

💻 Command Execution:
   /cmd <command>         - Execute command on selected client
   /cmd_all <cmd>         - Execute on ALL clients

📁 File Operations:
   /download <path>       - Download file from agent
   /upload <local> <remote> - Upload file to agent

🔧 Advanced Operations:
   /harvest               - Harvest credentials
   /elevate               - Elevate to admin (UAC)
   /persist <method>      - Establish persistence
   /persist_remove        - Remove persistence
   /beacon <int:jit>      - Configure beacon timing

🔐 Ransomware (if module loaded):
   /encrypt <path>        - Encrypt files
   /decrypt <path> <key>  - Decrypt files

ℹ️ Server:
   /help                  - Show help
   /exit, /quit           - Shutdown server
```

## Screenshots

The Team Client features a modern dark theme with:
- Connection status indicator
- Agent list with detailed information
- Scrollable console output
- Command input with history
- Quick action buttons

## Keyboard Shortcuts

- **Enter**: Send command
- **Up Arrow**: Previous command in history
- **Down Arrow**: Next command in history

## Security Notes

⚠️ **FOR AUTHORIZED SECURITY TESTING ONLY**

- SSH connection is encrypted
- Use key-based authentication for better security
- Never share credentials
- Always obtain proper authorization before testing

## Troubleshooting

### Connection Issues

1. **SSH Connection Failed**: Check host/port, firewall rules
2. **Authentication Failed**: Verify credentials/key path
3. **C2R2 Server Not Responding**: Ensure server is running on the specified port

### GUI Issues

1. **tkinter not found**: Install python3-tk package
2. **Font issues**: Install a monospace font like Consolas

## Development

To modify the client:

```bash
# Clone the repository
git clone https://github.com/G4sp4rCS/C2R2-v2.git
cd C2R2-v2/team-client

# Install dependencies
pip install -r requirements.txt

# Run in development
python c2r2_team_client.py
```

## License

MIT License - See [LICENSE](../LICENSE) for details.
