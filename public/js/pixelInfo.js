let pixelInfo = document.getElementById("pixelInfo");

let oldPixel = { x: -1, y: -1 };

async function updatePixelInfo() {
    if (pixel.x !== oldPixel.x || pixel.y !== oldPixel.y) {
        let response = await fetch(`/api/username/${pixel.x}/${pixel.y}`)
        if (response.ok) {
            let username = await response.text();
            pixelInfo.innerHTML = `(${pixel.x}, ${pixel.y}) ${username}`;
            oldPixel = { x: pixel.x, y: pixel.y };
        }
    }
}