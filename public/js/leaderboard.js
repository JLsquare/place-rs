let leaderboard = document.getElementById('leaderboardList');

function getLeaderboard() {
    fetch('/api/leaderboard')
        .then((response) => {
            return response.json();
        })
        .then((data) => {
            data.forEach((user) => {
                let userElement = document.createElement('div');
                userElement.className = 'leaderboard-user';

                let userName = document.createElement('div');
                userName.className = 'leaderboard-name';
                userName.innerHTML = user[0];

                let userPixels = document.createElement('div');
                userPixels.className = 'leaderboard-pixels';
                userPixels.innerHTML = user[1];

                userElement.appendChild(userName);
                userElement.appendChild(userPixels);

                leaderboard.appendChild(userElement);
            });
        })
}

getLeaderboard();