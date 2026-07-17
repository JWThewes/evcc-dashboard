/**
 * History view range selector controller.
 * Handles time-range button clicks and triggers chart refresh
 * via the charts.js API (echarts-integration contract).
 */
(function(){
  'use strict';
  var active='7d';
  function init(){
    var sel=document.querySelector('[data-testid="history-range-selector"]');
    if(!sel)return;
    var btns=sel.querySelectorAll('.range-btn');
    if(!btns.length)return;
    btns.forEach(function(btn){
      btn.addEventListener('click',function(){
        var slug=btn.getAttribute('data-range-slug');
        if(slug===active)return;
        active=slug;
        btns.forEach(function(b){b.setAttribute('aria-pressed',b===btn?'true':'false')});
        var secs=parseInt(btn.getAttribute('data-range-seconds'),10);
        var res=btn.getAttribute('data-range-resolution');
        var now=Math.floor(Date.now()/1000);
        var from=now-secs;
        var c=document.querySelector('[data-chart-type="energy"]');
        if(!c)return;
        var base=c.getAttribute('data-chart-url').split('/api/')[0];
        c.setAttribute('data-chart-url',base+'/api/chart/energy?from='+from+'&to='+now+'&resolution='+res);
        if(typeof window.refreshChart==='function')window.refreshChart(c);
      });
    });
  }
  if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',init);
  else init();
  document.addEventListener('htmx:afterSwap',function(e){
    if(e.detail.target&&e.detail.target.id==='view-content'){active='7d';init();}
  });
})();
