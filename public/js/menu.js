let menu = document.getElementById('menu');
let menuButton = document.getElementById('menuButton');

let loginDiv = document.getElementById('login');
let loginButton = document.getElementById('loginButton');
let loginLink = document.getElementById('loginLink');
let loginUsername = document.getElementById('loginUsername');
let loginPassword = document.getElementById('loginPassword');
let loginUsernameError = document.getElementById('loginUsernameError');
let loginPasswordError = document.getElementById('loginPasswordError');

let signupDiv = document.getElementById('signup');
let signupButton = document.getElementById('signupButton');
let signupLink = document.getElementById('signupLink');
let signupEmail = document.getElementById('signupEmail');
let signupUsername = document.getElementById('signupUsername');
let signupPassword = document.getElementById('signupPassword');
let signupEmailError = document.getElementById('signupEmailError');
let signupUsernameError = document.getElementById('signupUsernameError');
let signupPasswordError = document.getElementById('signupPasswordError');

let profileDiv = document.getElementById('profile');
let profilePlacedPixels = document.getElementById('profilePlacedPixels');
let profileRank = document.getElementById('profileRank');
let profileUsername = document.getElementById('profileUsername');
let profilePassword = document.getElementById('profilePassword');
let profileCurrentPassword = document.getElementById('profileCurrentPassword');
let profileUsernameError = document.getElementById('profileUsernameError');
let profilePasswordError = document.getElementById('profilePasswordError');
let profileCurrentPasswordError = document.getElementById('profileCurrentPasswordError');
let logoutButton = document.getElementById('logoutButton');
let saveButton = document.getElementById('saveButton');

function setupListeners() {
    menuButton.addEventListener('click', toggleMenu);
    signupLink.addEventListener('click', openSignup);
    loginLink.addEventListener('click', openLogin);
    loginButton.addEventListener('click', login);
    signupButton.addEventListener('click', signup);
    logoutButton.addEventListener('click', logout);
    saveButton.addEventListener('click', saveProfile);
}

function logout() {
    localStorage.removeItem('token');
    openLogin();
    toggleMenu();
    switchState("notConnected");
}

function saveProfile() {
    let token = localStorage.getItem("token");

    profileUsernameError.innerHTML = "";

    if(profileUsername.value.length < 3) {
        profileUsernameError.innerHTML = "Please enter at least 3 characters.";
        return;
    } else if(profileUsername.value.length > 15) {
        profileUsernameError.innerHTML = "Please enter at most 15 characters.";
        return;
    }

    profileCurrentPasswordError.innerHTML = "";

    if(!profileCurrentPassword.value) {
        profileCurrentPasswordError.innerHTML = "Please enter your current password.";
        return;
    }

    profilePasswordError.innerHTML = "";

    if(profilePassword.value) {
        if(profilePassword.value.length < 8) {
            profilePasswordError.innerHTML = "Please enter at least 8 characters.";
            return;
        } else if(profilePassword.value.length > 128) {
            profilePasswordError.innerHTML = "Please enter at most 128 characters.";
            return;
        }
    } else {
        profilePassword.value = "";
    }

    fetch('/api/profile/edit', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        },
        body: JSON.stringify({
            username: profileUsername.value,
            password: profilePassword.value,
            current_password: profileCurrentPassword.value
        })
    }).then(async response => {
        if (response.ok) {
            getProfile();
            toggleMenu();
            getLeaderboard();
        } else {
            if(response.status === 401) {
                profileCurrentPasswordError.innerHTML = "Invalid password.";
            } else if(response.status === 409) {
                profileUsernameError.innerHTML = "Username already taken.";
            } else if(response.status === 400) {
                profileUsernameError.innerHTML = "Username must be 3-15 characters long.";
                profilePasswordError.innerHTML = "Password must be 8-128 characters long.";
            }
        }
    });
}


function openLogin() {
    loginDiv.style.display = "flex";
    signupDiv.style.display = "none";
    profileDiv.style.display = "none";
}

function openSignup() {
    loginDiv.style.display = "none";
    signupDiv.style.display = "flex";
    profileDiv.style.display = "none";
}

function openProfile() {
    loginDiv.style.display = "none";
    signupDiv.style.display = "none";
    profileDiv.style.display = "flex";
}

function toggleMenu() {
    if(menu.style.display === "none") {
        menu.style.display = "flex";
    } else {
        menu.style.display = "none";
    }
}

function login() {
    if(loginUsername.value === "") {
        loginUsernameError.innerHTML = "Please enter an username.";
        return;
    }

    loginUsernameError.innerHTML = "";

    if(loginPassword.value === "") {
        loginPasswordError.innerHTML = "Please enter a password.";
        return;
    }


    fetch('/api/login', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            username: loginUsername.value,
            password: loginPassword.value
        })
    }).then(async response => {
        if (response.ok) {
            let token = await response.text();
            localStorage.setItem('token', token);
            openProfile();
            toggleMenu();
            switchState("palette");
        } else {
            loginUsernameError.innerHTML = "Invalid username or password.";
            loginPasswordError.innerHTML = "Invalid username or password.";
        }
    });
}

function signup() {
    signupEmail.value = signupEmail.value.trim();
    const emailRegex = /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$/;
    if(signupEmail.value === "" || !emailRegex.test(signupEmail.value)) {
        signupEmailError.innerHTML = "Please enter a valid email address.";
        return;
    }
    const ubsRegex = /^[a-z0-9.]+@(etud\.)?univ-ubs\.fr$/;
    if(!ubsRegex.test(signupEmail.value)) {
        signupEmailError.innerHTML = "Please enter a valid UBS email address.";
        return;
    }
    signupEmailError.innerHTML = "";

    signupUsername.value = signupUsername.value.trim();
    if(signupUsername.value.length < 3) {
        signupUsernameError.innerHTML = "Please enter at least 3 characters.";
        return;
    }
    signupUsernameError.innerHTML = "";

    signupPassword.value = signupPassword.value.trim();
    if(signupPassword.value.length < 8) {
        signupPasswordError.innerHTML = "Please enter at least 8 characters.";
        return;
    }
    signupPasswordError.innerHTML = "";

    fetch('/api/signup', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            email: signupEmail.value,
            username: signupUsername.value,
            password: signupPassword.value
        })
    }).then(async response => {
        if (response.ok) {
            openLogin();
            loginUsernameError.innerHTML = "Verification email sent.";
            loginPasswordError.innerHTML = "Please check your inbox.";
        } else {
            signupEmailError.innerHTML = "Something went wrong.";
            signupUsernameError.innerHTML = "Something went wrong.";
            signupPasswordError.innerHTML = "Something went wrong.";
            console.error("Error:", await response.text());
        }
    }).catch(error => {
        console.error("Error:", error);
    });
}

async function getProfile() {
    const token = localStorage.getItem("token");

    if (token === null) {
        openLogin();
        return;
    }

    let profileResponse = await fetch('/api/profile/me', {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        }
    });
    let countResponse = await fetch('/api/users/count', {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': token
        }
    });

    if(profileResponse.ok && countResponse.ok) {
        let profile = await profileResponse.json();
        let count = await countResponse.json();
        profilePlacedPixels.value = profile.score;
        profileRank.value = `${profile.rank} / ${count}`;
        profileUsername.value = profile.username;
        profilePassword.value = "";
        profileCurrentPassword.value = "";
        openProfile();
    } else {
        localStorage.removeItem('token');
        openLogin();
        console.error("Error:", await profileResponse.text());
    }
}


setupListeners();
toggleMenu();
openLogin();
getProfile();