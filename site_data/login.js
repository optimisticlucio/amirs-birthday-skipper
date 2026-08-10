

function update_button_pronouns() {
    let submitButton = document.getElementById("submit-button");

    let pronounSelected = document.getElementById("pronouns");

    switch (pronounSelected.value) {
        case "Male":
            submitButton.innerText = "התחבר";
            break;
        case "Female":
            submitButton.innerText = "התחברי";
            break;
        case "Mixed":
            submitButton.innerHTML = "התחבר.י";
            break;
    }
}

async function submit_data(targetUrl = window.location.pathname) {
    const userData = {
        name: document.getElementById("player-name").value,
        pronouns: document.getElementById("pronouns").value,
    };

    const presentationTitle = document.getElementById("presentation-name").value.trim();

    if (presentationTitle) {
        userData.presentation_title = presentationTitle;
    }

    const messageToSend = {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        credentials: "same-origin",
        body: JSON.stringify(userData)
    };

    console.log(`SENT: ${JSON.stringify(messageToSend)}`)

    const loginRequest = await fetch(targetUrl, messageToSend);

    if (loginRequest.status >= 400 && loginRequest.status < 600) {
        // TODO: Surface errors to user aswell
        let errorText = await loginRequest.text();
        console.log(`ERROR ${loginRequest.status}, ${loginRequest.statusText}: ${errorText}`);
        return;
    }
    // If there's a redirect, follow it, it means the login was successful.
    else if (loginRequest.redirected) {
        window.location.href = loginRequest.url;
    }
}