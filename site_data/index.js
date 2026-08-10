

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
    console.log("Connected to game server!");

    // Start listening only now that the handshake went through. The server sends
    // a SwitchPhase the moment it accepts us, but attaching here can't miss it:
    // the socket always dispatches `open` before any `message`.
    ws.onmessage = handleServerMessage;

    // From here on, a failure is a live connection dropping rather than a
    // handshake that never happened, so the two below replace the setup handler.
    ws.onerror = handleConnectionError;
    ws.onclose = handleConnectionClosed;
};

ws.onerror = () => {
    if (websocketCompleted) return;
    websocketCompleted = true;
    clearTimeout(timer);
    console.log(`Failed connecting to game server! Unknown error.`);
};

/// Something went wrong on an already-established connection. A `close` follows
/// this, so the cleanup belongs there, not here.
function handleConnectionError() {
    // TODO: surface to the user
    console.log("Game server connection errored.");
}

/// The connection is gone for good - the server drops it whenever the game ends
/// or the socket sends us something it considers unrecoverable.
function handleConnectionClosed(event) {
    // TODO: surface to the user, and decide whether to offer a reconnect
    console.log(`Disconnected from game server. (code ${event.code})`);
}

// --- Incoming: ServerMessage ------------------------------------------------
//
// Serde serializes these enums externally tagged, so a variant without data is
// just its name as a string, and one with data is a single-key object:
//
//   "InvalidJSON"
//   { "InvalidRequest": { "reason": "..." } }
//   { "InternalServerError": { "err": "..." } }
//   { "SwitchPhase": <PublicGamePhase> }
//
// PublicGamePhase is tagged the same way:
//
//   { "Setup":               { "connected_players": [Player] } }
//   { "SelectPresenter":     { "possible_presenters": [Player] } }
//   { "CurrentlyPresenting": { "presentation": Presentation,
//                              "players_who_skipped": [Player],
//                              "total_players": number } }
//   { "Results":             { "all_presentations": [Presentation] } }
//
// Player      = { id: number, name: string,
//                 presentation_title: string | null,
//                 pronouns: "Male" | "Female" | "Mixed" }
// Presentation = { presenter_id: number, name: string,
//                  start_time: string, end_time: string }  // RFC 3339 UTC

/// Splits an externally tagged enum into its variant name and payload.
/// The payload is null for variants that carry no data.
function unwrapTaggedEnum(message) {
    if (typeof message === "string") {
        return { variant: message, payload: null };
    }

    const keys = Object.keys(message);

    if (keys.length !== 1) {
        return null;
    }

    return { variant: keys[0], payload: message[keys[0]] };
}

/// Entry point for everything the server sends us. Attached in `ws.onopen`.
function handleServerMessage(event) {
    let serverMessage;

    try {
        serverMessage = JSON.parse(event.data);
    } catch (err) {
        console.log(`Received unparseable message from server: ${event.data}`);
        return;
    }

    const tagged = unwrapTaggedEnum(serverMessage);

    if (!tagged) {
        console.log(`Received malformed ServerMessage: ${event.data}`);
        return;
    }

    switch (tagged.variant) {
        case "SwitchPhase":
            handleSwitchPhase(tagged.payload);
            break;
        case "InvalidJSON":
            handleInvalidJSON();
            break;
        case "InvalidRequest":
            handleInvalidRequest(tagged.payload.reason);
            break;
        case "InternalServerError":
            handleInternalServerError(tagged.payload.err);
            break;
        default:
            console.log(`Unknown ServerMessage variant: ${tagged.variant}`);
    }
}

/// The game moved to a new phase; redraw the page to match it.
function handleSwitchPhase(publicGamePhase) {
    const tagged = unwrapTaggedEnum(publicGamePhase);

    if (!tagged) {
        console.log(`Received malformed PublicGamePhase: ${JSON.stringify(publicGamePhase)}`);
        return;
    }

    switch (tagged.variant) {
        case "Setup":
            renderSetup(tagged.payload.connected_players);
            break;
        case "SelectPresenter":
            renderSelectPresenter(tagged.payload.possible_presenters);
            break;
        case "CurrentlyPresenting":
            renderCurrentlyPresenting(
                tagged.payload.presentation,
                tagged.payload.players_who_skipped,
                tagged.payload.total_players,
            );
            break;
        case "Results":
            renderResults(tagged.payload.all_presentations);
            break;
        default:
            console.log(`Unknown PublicGamePhase variant: ${tagged.variant}`);
    }
}

/// The game hasn't started yet and players are logging in.
function renderSetup(connectedPlayers) {
    // TODO
    console.log("Phase: Setup", connectedPlayers);
}

/// The host needs to select who presents next.
function renderSelectPresenter(possiblePresenters) {
    // TODO
    console.log("Phase: SelectPresenter", possiblePresenters);
}

/// Someone is presenting right now.
function renderCurrentlyPresenting(presentation, playersWhoSkipped, totalPlayers) {
    // TODO
    console.log("Phase: CurrentlyPresenting", presentation, playersWhoSkipped, totalPlayers);
}

/// The game is over and we're showing off what happened.
function renderResults(allPresentations) {
    // TODO
    console.log("Phase: Results", allPresentations);
}

/// We sent the server something it couldn't parse. The server closes the socket
/// after this one.
function handleInvalidJSON() {
    // TODO: surface to the user
    console.log("Server rejected our message as invalid JSON.");
}

/// We asked for something we're not allowed to do, or that makes no sense in
/// this phase.
function handleInvalidRequest(reason) {
    // TODO: surface to the user
    console.log(`Invalid request: ${reason}`);
}

/// The server broke, not us.
function handleInternalServerError(err) {
    // TODO: surface to the user
    console.log(`Internal server error: ${err}`);
}

// --- Outgoing: UserMessage --------------------------------------------------
//
// Same tagging rules as above, going the other way.

function sendUserMessage(message) {
    if (ws.readyState !== WebSocket.OPEN) {
        console.log(`Tried sending while socket wasn't open: ${JSON.stringify(message)}`);
        return;
    }

    ws.send(JSON.stringify(message));
}

/// Host only. Starts the game.
function sendStartGame() {
    sendUserMessage("StartGame");
}

/// Host only. Picks who presents next.
function sendSelectPresenter(userId) {
    sendUserMessage({ SelectPresenter: { user_id: userId } });
}

/// Host or presenter only. Ends the presentation currently running.
function sendEndPresentation() {
    sendUserMessage("EndPresentation");
}

/// Anyone. Votes to skip the current presentation; doesn't skip it on its own.
function sendVoteToSkipPresentation() {
    sendUserMessage("VoteToSkipPresentation");
}

/// Host only. `newPercentage` is a fraction, not a percent - 0.75 means 75%.
function sendChangeSkipPercentage(newPercentage) {
    sendUserMessage({ ChangeSkipPercentage: { new_percentage: newPercentage } });
}
