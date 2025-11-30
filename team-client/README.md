# C2R2 Team Client

A graphical user interface (GUI) for operators to connect to C2R2 Command & Control servers.

## Architecture

Similar to Havoc C2's architecture:
- **C2R2 Server**: Runs on red team infrastructure with a dedicated API port
- **Team Client**: Operators connect from their machines via HTTP/WebSocket API
- **GUI Interface**: Visual display of connected agents, command execution, etc.

```
┌─────────────────────┐                    ┌─────────────────────┐
│   Operator Machine  │    HTTP/WebSocket  │  Red Team Server    │
│  ┌───────────────┐  │                    │  ┌───────────────┐  │
│  │ Team Client   │──┼────────────────────┼──│  C2R2 Server  │  │
│  │   (GUI)       │  │    (API Port)      │  │               │  │
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

- **API Connection**: Connect via HTTP/WebSocket to the C2R2 server API
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
- requests (HTTP client)
- websocket-client (WebSocket client)
- tkinter (usually included with Python)

## Installation

### Windows

```bash
# Install Python dependencies
pip install requests websocket-client

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
pip install requests websocket-client

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
- **Port 4444**: TLS port for agent connections
- **Port 5555**: HTTP/WebSocket API for team clients

### 2. Launch the Team Client

```bash
python c2r2_team_client.py
```

### 3. Connect via API

Enter the following details in the login screen:
- **Server Host**: IP address or hostname of the red team server
- **API Port**: API port (default 5555)
- **Username**: Your operator username (any name for identification)
- **API Password**: The password configured on the server (default: c2r2-secret)

### 4. Interact with Agents

Once connected, you'll see:
- **Left Panel**: List of connected agents (auto-updated via WebSocket)
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

## Server API Endpoints

The team client communicates with the server via these REST/WebSocket endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/auth/login` | POST | Authenticate and get token |
| `/api/auth/logout` | POST | Invalidate token |
| `/api/status` | GET | Server status |
| `/api/agents` | GET | List all agents |
| `/api/agents/:id` | GET | Get agent details |
| `/api/agents/:id/cmd` | POST | Send command to agent |
| `/api/agents/all/cmd` | POST | Send command to all agents |
| `/api/agents/:id/download` | POST | Request file download |
| `/api/agents/:id/harvest` | POST | Harvest credentials |
| `/api/agents/:id/persist` | POST | Set persistence |
| `/api/agents/:id/beacon` | POST | Configure beacon |
| `/api/agents/:id/elevate` | POST | Elevate to admin |
| `/api/events` | WS | WebSocket for real-time events |

## WebSocket Events

The `/api/events` WebSocket sends these event types:

- `AgentConnected` - New agent connected
- `AgentDisconnected` - Agent disconnected
- `AgentUpdated` - Agent info updated
- `CommandOutput` - Command output received
- `FileDownloaded` - File download completed
- `CredentialsHarvested` - Credentials harvested
- `RansomwareResult` - Ransomware operation result
- `ServerMessage` - Server info/warning/error

## Keyboard Shortcuts

- **Enter**: Send command
- **Up Arrow**: Previous command in history
- **Down Arrow**: Next command in history

## Security Notes

⚠️ **FOR AUTHORIZED SECURITY TESTING ONLY**

- API connection uses HTTP (consider using a reverse proxy with TLS for production)
- Use a strong API password
- Authentication tokens are session-based
- Never share credentials

## Legacy SSH Mode

The old SSH-based team client is still available as `c2r2_team_client_ssh.py` for backwards compatibility. The API-based client (`c2r2_team_client.py`) is recommended for new deployments.

## Troubleshooting

### Connection Issues

1. **Connection Refused**: Check that the server is running and the API port is correct
2. **Authentication Failed**: Verify the API password matches the server configuration
3. **WebSocket Disconnects**: Check firewall rules for persistent connections

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
