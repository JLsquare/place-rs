let colorPicker = document.getElementById("colorContainer");
let cooldownButton = document.getElementById("cooldownButton");
let notConnected = document.getElementById("notConnected");

let oldSelectedColorBlock = null;

function deselectColor() {
    if(oldSelectedColorBlock !== null) {
        oldSelectedColorBlock.className = 'color-block';
    }
    selectedColor = -1;
    cursor.style.backgroundColor = "transparent";
}

function initPalette() {
    colors = [
        "#6d001a",
        "#be0039",
        "#ff4500",
        "#ffa800",
        "#ffd635",
        "#fff8b8",
        "#00a368",
        "#00cc78",
        "#7eed56",
        "#00756f",
        "#009eaa",
        "#00ccc0",
        "#2450a4",
        "#3690ea",
        "#51e9f4",
        "#493ac1",
        "#6a5cff",
        "#94b3ff",
        "#811e9f",
        "#b44ac0",
        "#e4abff",
        "#de107f",
        "#ff3881",
        "#ff99aa",
        "#6d482f",
        "#9c6926",
        "#ffb470",
        "#000000",
        "#515252",
        "#898d90",
        "#d4d7d9",
        "#ffffff"
    ];

    colors.forEach((color) => {
        const colorBlock = document.createElement('button');
        colorBlock.className = 'color-block';
        colorBlock.style.backgroundColor = color;
        colorBlock.addEventListener('click', () => {
            deselectColor();
            selectedColor = colors.indexOf(color);
            colorBlock.className = 'color-block selected-color-block';
            oldSelectedColorBlock = colorBlock;
            cursor.style.backgroundColor = color + "bb";
        });
        colorPicker.appendChild(colorBlock);
    });
}

function switchState(state) {
    if(state === "cooldown") {
        cooldownButton.style.display = "block";
        colorPicker.style.display = "none";
        notConnected.style.display = "none";
    } else if(state === "palette") {
        cooldownButton.style.display = "none";
        colorPicker.style.display = "grid";
        notConnected.style.display = "none";
    } else if(state === "notConnected") {
        cooldownButton.style.display = "none";
        colorPicker.style.display = "none";
        notConnected.style.display = "block";
    }
}

async function getCooldown() {
    let token = localStorage.getItem("token");

    if(token === null) {
        switchState("notConnected");
        return;
    }

    let response = await fetch('/api/cooldown', {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        }
    });

    if(response.ok) {
        localCooldown = await response.json();
        updateCooldownDisplay();
    } else {
        switchState("notConnected");
        console.log(await response.text());
    }
}

function updateCooldownDisplay() {
    if (localCooldown <= 0) {
        switchState("palette");
        return;
    } else {
        switchState("cooldown");
    }
    cooldownButton.innerHTML = `${localCooldown} seconds`;
    localCooldown--;
    setTimeout(updateCooldownDisplay, 1000);
}