const socket = new WebSocket(`ws://${location.host}/live-reload`);

socket.addEventListener('message', (event) => {
    if (event.data === 'reload') {
        location.reload();
    }
});

socket.addEventListener('close', () => {
    console.log('Live reload connection lost. Reconnecting...');
    setTimeout(() => {
        location.reload();
    }, 1000);
});