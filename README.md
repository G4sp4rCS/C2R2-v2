# C2R2
C2 and Rat written in Rust

## Educational purpose only, do not use it for illegal activities.

## ToDo

- [x] Que no aparezca la consola del agente
- [ ] Crear un listener para tener multiples conexiones simultaneas con diferentes agentes
- [ ] Mejorar la ofuscación del agente

### Crear persistencia
- [ ] Cuando se ejecute el agente, que se copie a %APPDATA% y se añada al registro para que se ejecute al iniciar sesión o al iniciar el sistema.
- [ ] Que se pueda inyectar en un proceso legítimo (explorer.exe, svchost.exe, etc)


### Listener
- [ ] Crear un listener con sockets para tener multiples conexiones simultaneas con diferentes agentes
- [ ] Cuando se manda un comando que se haga de manera asíncrona para no bloquear la comunicación con el agente y además de una manera más sigilosa (threads, async/await, sleep, etc)
- Crear un servidor que se encargue de recibir las conexiones de los agentes y enviarles comandos y que este servidor se comunique con la interfaz C2 (Telegram bot, web, etc)

### Interfaz C2
- [ ] Crear una interfaz mediante Telegram bot que permita enviar comandos y recibir respuestas de los agentes
