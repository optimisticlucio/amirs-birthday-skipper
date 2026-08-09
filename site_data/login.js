

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

function submit_data() {
    // TODO: Surface errors to user
    // TODO: Implement
}