# C2R2 Team Client

A graphical user interface (GUI) for operators to connect to C2R2 Command & Control servers via SSH-tunneled API.

## Architecture

Similar to Havoc C2's architecture, all communication is 100% tunneled through SSH:
- **C2R2 Server**: Runs on red team infrastructure with a dedicated API port
- **Team Client**: Establishes SSH connection to the server
- **SSH Tunnel**: Port forwards the API port to localhost
- **API Communication**: All REST/WebSocket traffic goes through the encrypted SSH tunnel

```
┌─────────────────────┐                    ┌─────────────────────┐
│   Operator Machine  │                    │  Red Team Server    │
│  ┌───────────────┐  │      SSH (22)      │  ┌───────────────┐  │
│  │ Team Client   │──┼────────────────────┼──│  SSH Server   │  │
│  │   (GUI)       │  │                    │  └───────┬───────┘  │
│  └───────┬───────┘  │                    │          │          │
│          │          │                    │   Port Forward      │
│   localhost:10xxx   │                    │          │          │
│          │          │  ═══ SSH Tunnel ══>│          ▼          │
│          ▼          │                    │  ┌───────────────┐  │
│  ┌───────────────┐  │                    │  │  C2R2 Server  │  │
│  │ HTTP/WS API   │──┼── (through SSH) ──>│  │   API:5555    │  │
│  └───────────────┘  │                    │  └───────┬───────┘  │
└─────────────────────┘                    │          │          │
                                           │   🔐 TLS Encrypted  │
                                           │          ▼          │
                                           │  ┌───────────────┐  │
                                           │  │   Agents      │  │
                                           │  │  (Windows)    │  │
                                           │  └───────────────┘  │
                                           └─────────────────────┘
```

## Security Benefits

✅ **100% Tunneled**: All API traffic encrypted through SSH  
✅ **No Direct Exposure**: API port not exposed to the internet  
✅ **SSH Authentication**: Use SSH keys or passwords  
✅ **Double Encryption**: SSH tunnel + TLS for agent connections  

## Features

- **SSH Tunnel**: Automatic port forwarding through SSH
- **REST/WebSocket API**: Clean API communication through the tunnel
- **Real-time Updates**: WebSocket connection for live agent updates
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
- requests (HTTP client)
- websocket-client (WebSocket client)
- tkinter (usually included with Python)

## Installation

### Windows

```bash
# Install Python dependencies
pip install paramiko requests websocket-client

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
pip install paramiko requests websocket-client

# Run the client
python3 c2r2_team_client.py
```

## Usage

### 1. Start the C2R2 Server (on red team infrastructure)

```bash
# On the server (generates TLS certs first time)
cd c2r2-server
./target/release/c2r2-server --generate-certs
./target/release/c2r2-server --bind 0.0.0.0 --port 4444 --api-port 5555 --api-password your-secret-password
```

Server will listen on:
- **Port 4444**: TLS port for agent connections (exposed to agents)
- **Port 5555**: HTTP/WebSocket API (only accessible locally or via SSH tunnel)

### 2. Launch the Team Client

```bash
python c2r2_team_client.py
```

### 3. Connect via SSH Tunnel

Enter the following details in the login screen:

**SSH Connection:**
- **SSH Host**: IP address or hostname of the red team server
- **SSH Port**: SSH port (default 22)
- **SSH User**: Your SSH username on the server
- **SSH Password**: Your SSH password (or leave empty if using key)
- **SSH Key**: Path to your SSH private key (optional, recommended)

**API Connection:**
- **Remote API Port**: API port on the server (default 5555)
- **Operator Name**: Your operator name (for identification)
- **API Password**: The password configured on the server (--api-password flag)

### 4. Interact with Agents

Once connected, you'll see:
- **Left Panel**: List of connected agents (auto-updated via WebSocket through the tunnel)
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

💻 Command Execution:
   /cmd <command>         - Execute command on selected client
   /cmd_all <cmd>         - Execute on ALL clients

📁 File Operations:
   /download <path>       - Download file from agent

🔧 Advanced Operations:
   /harvest               - Harvest credentials
   /persist <method>      - Establish persistence (registry|task|wmi|startup)
   /beacon <int:jit>      - Configure beacon timing (e.g., 60:30)
   /elevate               - Elevate to admin (UAC prompt)

ℹ️ Other:
   /help                  - Show help
```

## How the SSH Tunnel Works

1. **Client connects to SSH server** on the red team infrastructure
2. **Port forward is established**: localhost:10xxx → server:5555 (API)
3. **API calls go through tunnel**: HTTP/WebSocket traffic is encrypted in SSH
4. **WebSocket for real-time events**: Also tunneled through SSH

This means:
- The API port (5555) doesn't need to be exposed to the internet
- Only SSH (22) and agent port (4444) need to be accessible
- All operator traffic is encrypted via SSH

## Keyboard Shortcuts

- **Enter**: Send command
- **Up Arrow**: Previous command in history
- **Down Arrow**: Next command in history

## Security Notes

⚠️ **FOR AUTHORIZED SECURITY TESTING ONLY**

- SSH connection encrypts all API traffic
- Use SSH key authentication for better security (recommended)
- Use a strong API password
- Never share credentials

## Troubleshooting

### SSH Connection Issues

1. **Connection Refused**: Check that SSH server is running on the specified port
2. **Authentication Failed**: Verify SSH credentials or key path
3. **Key Not Found**: Ensure the SSH key file exists and has correct permissions

### API Connection Issues

1. **Connection Refused after SSH**: Check that C2R2 server is running on the API port
2. **Authentication Failed**: Verify the API password matches server configuration

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
