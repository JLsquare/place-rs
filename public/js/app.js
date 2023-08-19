let colorPicker = document.getElementById("colorContainer");
let cooldownButton = document.getElementById("cooldownButton");
let cursor = document.getElementById("cursor");
let canvas = document.getElementById("canvas");
let ctx = canvas.getContext('2d');

ctx.imageSmoothingEnabled = false;
ctx.mozImageSmoothingEnabled = false;
ctx.webkitImageSmoothingEnabled = false;
ctx.msImageSmoothingEnabled = false;

const maxZoom = 128;
const minZoom = 0.75;
let currentZoom = 1;

let isDragging = false;
let recentlyDragged = false;
let lastPosition = { x: 0, y: 0 };
let offset = { x: 0, y: 0 };
let pixel = { x: 0, y: 0 };

let colors = [];
let selectedColor = 0;

let socket;
let localCooldown = 0;

function initSocket() {
    socket = new WebSocket(`ws://${window.location.host}/api/ws`);

    socket.onopen = function(e) {
        console.log("[open] Connection established");
    }

    socket.onmessage = function(event) {
        console.log(`[message] Data received from server: ${event.data}`);
        let data = JSON.parse(event.data);
        ctx.fillStyle = colors[data.color];
        ctx.fillRect(data.x, data.y, 1, 1);
    }

    socket.onclose = function(event) {
        if (event.wasClean) {
            console.log(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
        } else {
            console.log('[close] Connection died');
        }
    }

    socket.onerror = function(error) {
        console.log(`[error] ${error.message}`);
    }
}

function switchState(state) {
    if(state === "cooldown") {
        cooldownButton.style.display = "block";
        colorPicker.style.display = "none";
    } else if(state === "palette") {
        cooldownButton.style.display = "none";
        colorPicker.style.display = "grid";
    }
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
        const colorBlock = document.createElement('div');
        colorBlock.className = 'color-block';
        colorBlock.style.backgroundColor = color;
        colorBlock.addEventListener('click', () => {
            selectedColor = colors.indexOf(color);
            cursor.style.backgroundColor = color + "bb";
        });
        colorPicker.appendChild(colorBlock);
    });
}

function getCooldown() {
    let token = localStorage.getItem("token");

    fetch('/api/cooldown', {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        }
    })
        .then(response => response.json())
        .then(cooldownData => {
            let utcSeconds = Date.now() / 1000;
            localCooldown = Math.ceil(cooldownData - utcSeconds);
            updateCooldownDisplay();
        }
    );
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

function sendPixel() {
    let token = localStorage.getItem("token");

    fetch('/api/draw', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        },
        body: JSON.stringify({
            x: pixel.x,
            y: pixel.y,
            user: 0,
            color: selectedColor
        })
    }).then(async response => {
        if(response.ok){
            localCooldown = await response.json()
            updateCooldownDisplay();
        } else {
            console.log(await response.text());
            console.log(JSON.stringify({
                x: pixel.x,
                y: pixel.y,
                user: 0,
                color: selectedColor
            }))
        }
    });
}

function getGrid() {
    fetch('/api/size')
        .then(response => response.json())
        .then(sizeData => {
            canvas.width = sizeData[0];
            canvas.height = sizeData[1]
            fetch('/api/pixels')
                .then(response => response.arrayBuffer())
                .then(pixelsData => {
                    const bytes = new Uint8Array(pixelsData);

                    for (let x = 0; x < sizeData[0]; x++) {
                        for (let y = 0; y < sizeData[1]; y++) {
                            ctx.fillStyle = colors[bytes[x * sizeData[1] + y]];
                            ctx.fillRect(x, y, 1, 1);
                        }
                    }
                })
        }
    );
}

function cursorPosition(event){
    const canvasBounds = canvas.getBoundingClientRect();

    let x = Math.floor((event.clientX - canvasBounds.left) / currentZoom);
    let y = Math.floor((event.clientY - canvasBounds.top) / currentZoom);
    pixel = { x, y };

    cursor.style.left = `${canvasBounds.left + x * currentZoom - currentZoom * 0.1}px`;
    cursor.style.top = `${canvasBounds.top + y * currentZoom - currentZoom * 0.1}px`;
    cursor.style.width = `${currentZoom * 1.2}px`;
    cursor.style.height = `${currentZoom * 1.2}px`;
}

canvas.addEventListener('wheel', (event) => {
    event.preventDefault();

    let prevZoom = currentZoom;
    currentZoom = event.wheelDelta > 0 ? currentZoom * 1.1 : currentZoom / 1.1;
    currentZoom = Math.min(Math.max(currentZoom, minZoom), maxZoom);

    offset.x -= (canvas.width / 2 - offset.x) * (currentZoom - prevZoom) / prevZoom;
    offset.y -= (canvas.height / 2 - offset.y) * (currentZoom - prevZoom) / prevZoom;

    canvas.style.transform = `translate(${offset.x}px, ${offset.y}px) scale(${currentZoom})`;

    cursorPosition(event);
});

canvas.addEventListener('mousedown', (event) => {
    isDragging = true;
    lastPosition.x = event.clientX;
    lastPosition.y = event.clientY;
});

canvas.addEventListener('mouseup', () => {
    if (isDragging) {
        setTimeout(() => {
            recentlyDragged = false;
        }, 100);
    }
    isDragging = false;
});

canvas.addEventListener('mousemove', (event) => {
    if (isDragging) {
        recentlyDragged = true;

        const dx = event.clientX - lastPosition.x;
        const dy = event.clientY - lastPosition.y;

        offset.x += dx;
        offset.y += dy;

        canvas.style.transform = `translate(${offset.x}px, ${offset.y}px) scale(${currentZoom})`;

        lastPosition.x = event.clientX;
        lastPosition.y = event.clientY;
    }

    cursorPosition(event);
});

canvas.addEventListener('click', () => {
    if (!recentlyDragged) {
        sendPixel();
    }
});

initPalette();
getGrid();
switchState("palette");
initSocket();
getCooldown();