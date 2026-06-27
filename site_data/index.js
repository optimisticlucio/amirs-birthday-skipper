

console.log("Connecting to game server...");

const proto = location.protocol === "https:" ? "wss:" : "ws:";
const ws = new WebSocket(`${proto}//${location.host}/websocket`);

let websocketCompleted = false;

const timer = setTimeout(() => {
    if (!websocketCompleted) {
        websocketCompleted = true;
        ws.close();
        console.log("Failed connecting to server! (timeout)");
    }
}, 1000);

ws.onopen = () => {
    if (websocketCompleted) return;
    websocketCompleted = true;
    clearTimeout(timer);
    // continue your code here
    console.log("Connected to game server!");
};

ws.onerror = () => {
    if (websocketCompleted) return;
    websocketCompleted = true;
    clearTimeout(timer);
    console.log(`Failed connecting to game server! Unknown error.`);
};