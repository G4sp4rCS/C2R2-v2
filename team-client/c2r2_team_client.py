#!/usr/bin/env python3
"""
C2R2 Team Client - GUI Interface for C2R2 Server via SSH

This application provides a graphical interface for operators to connect
to a C2R2 server running on remote infrastructure via SSH tunnel.

Similar to Havoc Team Client architecture:
- Server runs on red team infrastructure
- Operators connect via SSH from their machines
- GUI displays connected agents, allows command execution, etc.
"""

import os
import sys
import time
import queue
import socket
import threading
import re
from datetime import datetime

# Tkinter imports (cross-platform GUI)
import tkinter as tk
from tkinter import ttk, messagebox, scrolledtext, filedialog

# SSH library
try:
    import paramiko
except ImportError:
    print("Error: paramiko is required. Install with: pip install paramiko")
    sys.exit(1)


class SSHConnection:
    """Manages SSH connection and port forwarding to C2R2 server."""
    
    def __init__(self):
        self.ssh_client = None
        self.channel = None
        self.connected = False
        self.server_info = {}
        self.transport = None
        self.forwarded_port = None
        self.local_socket = None
        
    def connect(self, host, ssh_port, username, password=None, key_path=None, c2_port=4444):
        """
        Connect to the SSH server and set up port forwarding to C2R2 server.
        
        Args:
            host: SSH server hostname/IP
            ssh_port: SSH port (default 22)
            username: SSH username
            password: SSH password (if not using key)
            key_path: Path to SSH private key (if not using password)
            c2_port: Port where C2R2 server is running (default 4444)
        """
        try:
            self.ssh_client = paramiko.SSHClient()
            self.ssh_client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
            
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
                raise ValueError("Either password or SSH key is required")
            
            self.ssh_client.connect(**connect_params)
            self.transport = self.ssh_client.get_transport()
            
            # Store connection info
            self.server_info = {
                'host': host,
                'ssh_port': ssh_port,
                'username': username,
                'c2_port': c2_port,
            }
            
            self.connected = True
            return True, "Connected successfully"
            
        except paramiko.AuthenticationException:
            return False, "Authentication failed. Check username/password/key."
        except paramiko.SSHException as e:
            return False, f"SSH error: {str(e)}"
        except socket.error as e:
            return False, f"Connection error: {str(e)}"
        except Exception as e:
            return False, f"Error: {str(e)}"
    
    def execute_command(self, command):
        """Execute a command on the remote server."""
        if not self.connected or not self.ssh_client:
            return False, "Not connected"
        
        try:
            stdin, stdout, stderr = self.ssh_client.exec_command(command, timeout=60)
            output = stdout.read().decode('utf-8', errors='replace')
            error = stderr.read().decode('utf-8', errors='replace')
            
            if error and not output:
                return False, error
            return True, output + error
            
        except Exception as e:
            return False, f"Error executing command: {str(e)}"
    
    def start_c2_interaction(self, c2_path, c2_port, bind_addr="0.0.0.0"):
        """
        Start an interactive session with the C2R2 server.
        Returns a channel for bidirectional communication.
        
        Args:
            c2_path: Path to the c2r2-server binary on the remote server
            c2_port: Port for the C2R2 server
            bind_addr: Address to bind the C2 server to
        """
        if not self.connected or not self.ssh_client:
            return False, "Not connected", None
        
        try:
            # Get a shell channel for interactive use
            self.channel = self.ssh_client.invoke_shell(
                term='xterm',
                width=200,
                height=50
            )
            self.channel.settimeout(0.5)
            
            # Wait for shell to be ready
            time.sleep(0.5)
            
            # Clear initial banner/prompt
            try:
                while self.channel.recv_ready():
                    self.channel.recv(4096)
            except Exception:
                pass
            
            # Start C2R2 server if provided path
            if c2_path:
                # First check if server is already running
                cmd = f"pgrep -f 'c2r2-server.*{c2_port}' >/dev/null 2>&1 && echo 'RUNNING' || echo 'NOT_RUNNING'\n"
                self.channel.send(cmd)
                time.sleep(0.5)
                
                response = ""
                try:
                    while self.channel.recv_ready():
                        response += self.channel.recv(4096).decode('utf-8', errors='replace')
                except Exception:
                    pass
                
                # If not running, start it
                if 'NOT_RUNNING' in response:
                    # Start the server
                    start_cmd = f"{c2_path} --bind {bind_addr} --port {c2_port}\n"
                    self.channel.send(start_cmd)
                    time.sleep(2)  # Wait for server to start
            
            return True, "Interactive session started", self.channel
            
        except Exception as e:
            return False, f"Error starting interactive session: {str(e)}", None
    
    def send_to_channel(self, data):
        """Send data to the interactive channel."""
        if self.channel:
            try:
                self.channel.send(data)
                return True
            except Exception as e:
                return False
        return False
    
    def recv_from_channel(self, size=4096):
        """Receive data from the interactive channel."""
        if self.channel:
            try:
                if self.channel.recv_ready():
                    return self.channel.recv(size).decode('utf-8', errors='replace')
            except socket.timeout:
                pass
            except Exception:
                pass
        return ""
    
    def disconnect(self):
        """Disconnect from SSH server."""
        try:
            if self.channel:
                self.channel.close()
            if self.ssh_client:
                self.ssh_client.close()
        except Exception:
            pass
        finally:
            self.connected = False
            self.channel = None
            self.ssh_client = None
            self.transport = None


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
        
        # Initialize connection
        self.ssh = SSHConnection()
        self.output_queue = queue.Queue()
        self.running = True
        self.selected_client = None
        self.agents = {}  # Dictionary to store connected agents
        
        # Create UI
        self._setup_styles()
        self._create_menu()
        self._create_login_frame()
        self._create_main_frame()
        
        # Start with login frame visible
        self.main_frame.pack_forget()
        self.login_frame.pack(fill=tk.BOTH, expand=True)
        
        # Start output processor
        self.root.after(100, self._process_output_queue)
        
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
            text="Connect to C2R2 Server via SSH",
            style='Dark.TLabel',
            font=('Consolas', 12)
        )
        subtitle.pack(pady=(0, 20))
        
        # Connection form
        form_frame = ttk.Frame(center_frame, style='Panel.TFrame', padding=30)
        form_frame.pack(padx=20, pady=10)
        
        # SSH Host
        row = 0
        ttk.Label(form_frame, text="SSH Host:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.host_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.host_entry.grid(row=row, column=1, padx=5, pady=8)
        self.host_entry.insert(0, "192.168.1.100")
        
        # SSH Port
        row += 1
        ttk.Label(form_frame, text="SSH Port:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.ssh_port_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.ssh_port_entry.grid(row=row, column=1, padx=5, pady=8)
        self.ssh_port_entry.insert(0, "22")
        
        # Username
        row += 1
        ttk.Label(form_frame, text="Username:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.username_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.username_entry.grid(row=row, column=1, padx=5, pady=8)
        self.username_entry.insert(0, "operator")
        
        # Password
        row += 1
        ttk.Label(form_frame, text="Password:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.password_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry', show='*')
        self.password_entry.grid(row=row, column=1, padx=5, pady=8)
        
        # SSH Key (optional)
        row += 1
        ttk.Label(form_frame, text="SSH Key (optional):", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        key_frame = ttk.Frame(form_frame, style='Panel.TFrame')
        key_frame.grid(row=row, column=1, padx=5, pady=8, sticky='w')
        self.key_entry = ttk.Entry(key_frame, width=30, style='Dark.TEntry')
        self.key_entry.pack(side=tk.LEFT)
        browse_btn = ttk.Button(key_frame, text="Browse", command=self._browse_key)
        browse_btn.pack(side=tk.LEFT, padx=(5, 0))
        
        # C2 Server Port
        row += 1
        ttk.Label(form_frame, text="C2 Server Port:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.c2_port_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.c2_port_entry.grid(row=row, column=1, padx=5, pady=8)
        self.c2_port_entry.insert(0, "4444")
        
        # C2 Server Path (optional)
        row += 1
        ttk.Label(form_frame, text="C2 Binary Path:", style='Panel.TLabel').grid(
            row=row, column=0, sticky='e', padx=5, pady=8)
        self.c2_path_entry = ttk.Entry(form_frame, width=40, style='Dark.TEntry')
        self.c2_path_entry.grid(row=row, column=1, padx=5, pady=8)
        self.c2_path_entry.insert(0, "")
        
        row += 1
        hint_label = ttk.Label(
            form_frame, 
            text="(Leave empty if C2R2 server is already running)",
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
        self.cmd_history = []
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
    
    def _browse_key(self):
        """Open file dialog to select SSH key."""
        filename = filedialog.askopenfilename(
            title="Select SSH Private Key",
            filetypes=[("All files", "*"), ("PEM files", "*.pem"), ("Key files", "*.key")]
        )
        if filename:
            self.key_entry.delete(0, tk.END)
            self.key_entry.insert(0, filename)
    
    def _connect(self):
        """Handle connection to SSH server."""
        host = self.host_entry.get().strip()
        ssh_port = self.ssh_port_entry.get().strip()
        username = self.username_entry.get().strip()
        password = self.password_entry.get()
        key_path = self.key_entry.get().strip()
        c2_port = self.c2_port_entry.get().strip()
        c2_path = self.c2_path_entry.get().strip()
        
        if not host or not username:
            self.login_status.config(text="❌ Host and username are required", foreground=self.colors['error'])
            return
        
        if not password and not key_path:
            self.login_status.config(text="❌ Password or SSH key is required", foreground=self.colors['error'])
            return
        
        self.login_status.config(text="⏳ Connecting...", foreground=self.colors['warning'])
        self.connect_btn.config(state=tk.DISABLED)
        self.root.update()
        
        # Connect in a separate thread to avoid blocking UI
        def connect_thread():
            success, message = self.ssh.connect(
                host, ssh_port, username, 
                password=password if password else None,
                key_path=key_path if key_path else None,
                c2_port=c2_port
            )
            
            if success:
                # Start interactive session with C2R2 server
                c2_path_param = c2_path if c2_path else None
                ok, msg, _ = self.ssh.start_c2_interaction(c2_path_param, int(c2_port))
                
                if ok:
                    self.root.after(0, lambda: self._on_connect_success(host, c2_port))
                else:
                    self.root.after(0, lambda: self._on_connect_fail(msg))
            else:
                self.root.after(0, lambda: self._on_connect_fail(message))
        
        thread = threading.Thread(target=connect_thread, daemon=True)
        thread.start()
    
    def _on_connect_success(self, host, port):
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
        
        # Start receiver thread
        self.receiver_thread = threading.Thread(target=self._receive_data, daemon=True)
        self.receiver_thread.start()
        
        # Request initial client list
        self.root.after(1000, lambda: self._send_raw_command("/list\n"))
    
    def _on_connect_fail(self, message):
        """Handle connection failure."""
        self.login_status.config(text=f"❌ {message}", foreground=self.colors['error'])
        self.connect_btn.config(state=tk.NORMAL)
    
    def _disconnect(self):
        """Disconnect from SSH server."""
        self.ssh.disconnect()
        self.main_frame.pack_forget()
        self.login_frame.pack(fill=tk.BOTH, expand=True)
        self.login_status.config(text="")
        self.connect_btn.config(state=tk.NORMAL)
        
        # Clear agents list
        for item in self.agents_tree.get_children():
            self.agents_tree.delete(item)
        self.agents.clear()
        self.selected_client = None
        self._update_prompt()
    
    def _receive_data(self):
        """Thread to receive data from SSH channel."""
        buffer = ""
        while self.running and self.ssh.connected:
            try:
                data = self.ssh.recv_from_channel()
                if data:
                    buffer += data
                    
                    # Process complete lines
                    while '\n' in buffer:
                        line, buffer = buffer.split('\n', 1)
                        self.output_queue.put(line)
                
                time.sleep(0.1)
            except Exception as e:
                if self.running:
                    self.output_queue.put(f"Error receiving data: {e}")
                break
    
    def _process_output_queue(self):
        """Process output from the queue (runs in main thread)."""
        try:
            while True:
                line = self.output_queue.get_nowait()
                self._process_line(line)
        except queue.Empty:
            pass
        
        if self.running:
            self.root.after(100, self._process_output_queue)
    
    def _process_line(self, line):
        """Process a line of output from the server."""
        # Strip ANSI codes for processing but keep for display
        clean_line = self._strip_ansi(line)
        
        # Parse client list output
        if '│' in clean_line and clean_line.count('│') >= 6:
            # This might be a table row
            parts = [p.strip() for p in clean_line.split('│')]
            if len(parts) >= 7:
                try:
                    # Try to parse as agent info
                    agent_id = parts[1].strip()
                    if agent_id.isdigit():
                        self._update_agent({
                            'id': agent_id,
                            'addr': parts[2].strip(),
                            'hostname': parts[3].strip(),
                            'username': parts[4].strip(),
                            'os': parts[5].strip(),
                            'privileges': parts[6].strip(),
                        })
                except (IndexError, ValueError):
                    pass
        
        # Log to console
        self._log_console(line + '\n')
    
    def _strip_ansi(self, text):
        """Remove ANSI escape codes from text."""
        ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
        return ansi_escape.sub('', text)
    
    def _log_console(self, text, tag=None):
        """Log text to the console output."""
        self.console_output.config(state=tk.NORMAL)
        if tag:
            self.console_output.insert(tk.END, text, tag)
        else:
            self.console_output.insert(tk.END, text)
        self.console_output.see(tk.END)
        self.console_output.config(state=tk.DISABLED)
    
    def _update_agent(self, agent_info):
        """Update or add an agent to the tree."""
        agent_id = agent_info['id']
        
        # Check if agent already exists
        for item in self.agents_tree.get_children():
            values = self.agents_tree.item(item)['values']
            if str(values[0]) == str(agent_id):
                # Update existing
                self.agents_tree.item(item, values=(
                    agent_id,
                    agent_info.get('hostname', agent_info.get('addr', '...')),
                    agent_info.get('username', '...'),
                    agent_info.get('os', '...'),
                    agent_info.get('privileges', '...')
                ))
                self.agents[agent_id] = agent_info
                return
        
        # Add new agent
        self.agents_tree.insert('', tk.END, values=(
            agent_id,
            agent_info.get('hostname', agent_info.get('addr', '...')),
            agent_info.get('username', '...'),
            agent_info.get('os', '...'),
            agent_info.get('privileges', '...')
        ))
        self.agents[agent_id] = agent_info
    
    def _on_agent_select(self, event):
        """Handle agent selection in treeview."""
        selection = self.agents_tree.selection()
        if selection:
            item = selection[0]
            values = self.agents_tree.item(item)['values']
            agent_id = str(values[0])
            
            # Select the agent
            self._send_raw_command(f"/select {agent_id}\n")
            self.selected_client = agent_id
            self._update_prompt()
    
    def _on_agent_double_click(self, event):
        """Handle double-click on agent."""
        selection = self.agents_tree.selection()
        if selection:
            item = selection[0]
            values = self.agents_tree.item(item)['values']
            agent_id = str(values[0])
            
            # Show agent info
            self._send_raw_command(f"/info {agent_id}\n")
    
    def _update_prompt(self):
        """Update the command prompt."""
        if self.selected_client:
            self.selected_label.config(text=f"C2R2[{self.selected_client}]>")
        else:
            self.selected_label.config(text="C2R2>")
    
    def _send_command(self, event):
        """Send command from the entry field."""
        cmd = self.cmd_entry.get().strip()
        if cmd:
            # Add to history
            self.cmd_history.append(cmd)
            self.history_index = len(self.cmd_history)
            
            # Clear entry
            self.cmd_entry.delete(0, tk.END)
            
            # Send command
            self._send_raw_command(cmd + '\n')
            
            # Handle local commands
            if cmd.startswith('/select '):
                try:
                    parts = cmd.split()
                    if len(parts) >= 2:
                        self.selected_client = parts[1]
                        self._update_prompt()
                except Exception:
                    pass
            elif cmd == '/deselect':
                self.selected_client = None
                self._update_prompt()
    
    def _send_raw_command(self, cmd):
        """Send raw command to SSH channel."""
        if self.ssh.connected:
            self.ssh.send_to_channel(cmd)
            self._log_console(f">>> {cmd}", 'prompt')
    
    def _quick_command(self, cmd):
        """Execute a quick action command."""
        self._send_raw_command(cmd + '\n')
    
    def _refresh_agents(self):
        """Refresh the agents list."""
        # Clear current list
        for item in self.agents_tree.get_children():
            self.agents_tree.delete(item)
        self.agents.clear()
        
        # Request new list
        self._send_raw_command("/list\n")
    
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
            "C2R2 Team Client v1.0\n\n"
            "A graphical interface for connecting to\n"
            "C2R2 Command & Control servers via SSH.\n\n"
            "Similar to Havoc Team Client architecture:\n"
            "- Server runs on red team infrastructure\n"
            "- Operators connect via SSH\n"
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
"""
        # Create a new window for help
        help_window = tk.Toplevel(self.root)
        help_window.title("C2R2 Commands Help")
        help_window.geometry("500x600")
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
        self.ssh.disconnect()
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
