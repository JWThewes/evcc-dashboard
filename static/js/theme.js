/**
 * Theme + Navigation — evcc Dashboard (foundation unit)
 * Theme toggle with dual persistence, tab keyboard nav, htmx integration
 */
(function(){
"use strict";
var SK="evcc-theme",CK="evcc_theme";

function getTheme(){
  var c=getCookie(CK);
  if(c==="light"||c==="dark")return c;
  try{var s=localStorage.getItem(SK);if(s==="light"||s==="dark")return s;}catch(e){}
  return"dark";
}

function setTheme(t){
  if(t!=="dark"&&t!=="light")t="dark";
  document.documentElement.setAttribute("data-theme",t);
  try{localStorage.setItem(SK,t);}catch(e){}
  setCookie(CK,t,365);
  updateToggles(t);
}

function getCookie(n){
  var p=document.cookie.split(";");
  for(var i=0;i<p.length;i++){var c=p[i].trim();if(c.startsWith(n+"="))return c.substring(n.length+1);}
  return null;
}

function setCookie(n,v,d){
  var e=new Date(Date.now()+d*864e5).toUTCString();
  document.cookie=n+"="+v+";path=/;SameSite=Lax;expires="+e;
}

function updateToggles(t){
  var btns=document.querySelectorAll("[data-testid='theme-toggle'] button");
  btns.forEach(function(b){b.setAttribute("aria-pressed",b.getAttribute("data-theme-value")===t?"true":"false");});
}

function initTabNav(){
  var tl=document.querySelector('[role="tablist"]');
  if(!tl)return;
  tl.addEventListener("keydown",function(e){
    var tabs=Array.from(tl.querySelectorAll('[role="tab"]'));
    var cur=tabs.indexOf(document.activeElement);
    if(cur<0)return;
    var next=-1;
    if(e.key==="ArrowRight"||e.key==="ArrowDown")next=(cur+1)%tabs.length;
    else if(e.key==="ArrowLeft"||e.key==="ArrowUp")next=(cur-1+tabs.length)%tabs.length;
    else if(e.key==="Home")next=0;
    else if(e.key==="End")next=tabs.length-1;
    if(next>=0){e.preventDefault();tabs[cur].setAttribute("tabindex","-1");tabs[next].setAttribute("tabindex","0");tabs[next].focus();}
  });
}

function updateActiveTab(){
  var path=window.location.pathname;
  document.querySelectorAll('[role="tab"]').forEach(function(tab){
    var href=tab.getAttribute("href")||"";
    var active=path===href||path.endsWith(href)||(href.endsWith("/overview")&&(path==="/"||path.endsWith("/")));
    tab.setAttribute("aria-selected",active?"true":"false");
    tab.setAttribute("tabindex",active?"0":"-1");
  });
}

function init(){
  setTheme(getTheme());
  initTabNav();
  updateActiveTab();
  document.addEventListener("click",function(e){
    var btn=e.target.closest("[data-theme-value]");
    if(btn){e.preventDefault();setTheme(btn.getAttribute("data-theme-value"));}
  });
  document.addEventListener("htmx:afterSettle",updateActiveTab);
  window.addEventListener("popstate",updateActiveTab);
}

if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",init);
else init();
})();
