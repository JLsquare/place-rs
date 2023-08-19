const BASE_URL = `${window.location.protocol}//${window.location.host}`;

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

function setupListeners() {
    menuButton.addEventListener('click', toggleMenu);
    signupLink.addEventListener('click', openSignup);
    loginLink.addEventListener('click', openLogin);
    loginButton.addEventListener('click', login);
    signupButton.addEventListener('click', signup);
}

function openLogin() {
    loginDiv.style.display = "flex";
    signupDiv.style.display = "none";
}

function openSignup() {
    loginDiv.style.display = "none";
    signupDiv.style.display = "flex";
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
            toggleMenu();
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
        }
    });
}

setupListeners();
toggleMenu();
openLogin();