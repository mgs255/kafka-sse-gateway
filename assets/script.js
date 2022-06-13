var est1 = new EventSource('sse/subscribe/t1');
var est2 = new EventSource('sse/subscribe/t2');
var est3 = new EventSource('sse/subscribe/t3');
var estt1a = new EventSource('sse/subscribe/t1');

console.log('Starting consumers');

function getTime() {
    return new Date().toLocaleString();
}

est1.onmessage = function(event) {
    console.log(`Message from t1 ${event.data} ${getTime()}`);
}

est1.onerror = function()
{
    console.log("Closing t1");
    est1.close();
}

estt1a.onmessage = function(event) {
    console.log(`Message from t1a ${event.data} ${getTime()}`);
}

estt1a.onerror = function()
{
    console.log("Closing t1");
    estt1a.close();
}

est2.onmessage = function(event) {
    console.log(`Message from t2 ${event.data} ${getTime()}`);
}

est2.onerror = function()
{
    console.log("Closing t2");
    est2.close();
}

est3.onmessage = function(event) {
    console.log(`Message from t3 ${event.data} ${getTime()}`);
}

est3.onerror = function()
{
    console.log("Closing t3");
    est3.close();
}

setTimeout(()=>{ console.log("Closing consumer t3"); est3.close();}, 120000);
setTimeout(()=>{ console.log("Closing consumer t2"); est2.close();}, 145000);
setTimeout(()=>{ console.log("Closing consumer t1a"); estt1a.close();}, 160000);