(function () {
  'use strict';

  var basePath = document.documentElement.getAttribute('data-base-path') || '';
  var endpoint = basePath + '/api/events';
  var energyFlowCard = null;
  var batteryCard = null;
  var banner = null;
  var es = null;
  var retryDelay = 1000;
  var maxRetryDelay = 30000;
  var consecutiveFailures = 0;
  var maxFailures = 3;
  var retryTimer = null;

  function getCards() {
    energyFlowCard = document.getElementById('sse-energy-flow');
    batteryCard = document.getElementById('sse-battery');
    banner = document.getElementById('sse-reconnect-banner');
  }

  function showBanner() {
    if (banner) banner.classList.add('visible');
  }

  function hideBanner() {
    if (banner) banner.classList.remove('visible');
  }

  function fallbackToPolling() {
    if (energyFlowCard) {
      energyFlowCard.setAttribute('hx-get', basePath + '/partials/energy-flow');
      energyFlowCard.setAttribute('hx-trigger', 'every 2s');
      energyFlowCard.setAttribute('hx-swap', 'innerHTML');
    }
    if (batteryCard) {
      batteryCard.setAttribute('hx-get', basePath + '/partials/battery');
      batteryCard.setAttribute('hx-trigger', 'every 2s');
      batteryCard.setAttribute('hx-swap', 'innerHTML');
    }
    if (typeof htmx !== 'undefined') {
      htmx.process(document.body);
    }
    hideBanner();
  }

  function connect() {
    if (es) {
      es.close();
      es = null;
    }
    if (typeof EventSource === 'undefined') {
      fallbackToPolling();
      return;
    }

    es = new EventSource(endpoint);

    es.onopen = function () {
      retryDelay = 1000;
      consecutiveFailures = 0;
      hideBanner();
    };

    es.onmessage = function (event) {
      try {
        var msg = JSON.parse(event.data);
        if (msg.type === 'energy-flow' && energyFlowCard) {
          energyFlowCard.innerHTML = msg.data;
        } else if (msg.type === 'battery' && batteryCard) {
          batteryCard.innerHTML = msg.data;
        }
      } catch (e) {
        // ignore malformed messages
      }
    };

    es.onerror = function () {
      es.close();
      es = null;
      consecutiveFailures++;
      if (consecutiveFailures >= maxFailures) {
        fallbackToPolling();
        return;
      }
      showBanner();
      retryTimer = setTimeout(function () {
        connect();
      }, retryDelay);
      retryDelay = Math.min(retryDelay * 2, maxRetryDelay);
    };
  }

  function disconnect() {
    if (retryTimer) {
      clearTimeout(retryTimer);
      retryTimer = null;
    }
    if (es) {
      es.close();
      es = null;
    }
  }

  function handleVisibility() {
    if (document.hidden) {
      disconnect();
    } else {
      connect();
    }
  }

  function init() {
    getCards();
    if (!energyFlowCard && !batteryCard) return;
    document.addEventListener('visibilitychange', handleVisibility);
    connect();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
