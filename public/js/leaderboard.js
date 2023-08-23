let leaderboard = document.getElementById('leaderboardList');
let reloadButton = document.getElementById('reloadButton');

reloadButton.addEventListener('click', getLeaderboard);

function getLeaderboard() {
    reloadButton.classList.toggle('rotate')
    reloadButton.onanimationend = () => {
        reloadButton.classList.toggle('rotate')
    }

    fetch('/api/leaderboard')
        .then((response) => {
            return response.json();
        })
        .then((data) => {
            leaderboard.innerHTML = '';
            data.forEach(user => {
                let userElement = document.createElement('div');
                userElement.className = 'leaderboard-user';

                let userName = document.createElement('p');
                userName.className = 'leaderboard-name';
                userName.innerHTML = `${user.rank}. ${user.username}`;

                let userPixels = document.createElement('p');
                userPixels.className = 'leaderboard-pixels';
                userPixels.innerHTML = user.score;

                userElement.appendChild(userName);
                userElement.appendChild(userPixels);

                leaderboard.appendChild(userElement);
            });
        })
}

getLeaderboard();