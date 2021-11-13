
function main() {
    let screen = document.getElementById("screen");
    let urlbar = document.getElementById("url");

    var url = new URL(window.location);
    url.pathname = 'ws';
    url.protocol = (url.protocol === 'https:') ? 'wss' : 'ws';

    let ws = new WebSocket(url);
    ws.onmessage = function onMessage(message) {
        console.log('got message');
    
        let {data} = message;

        if (typeof data === "string") {
            urlbar.setAttribute("value", data);
        } else {
            let url = URL.createObjectURL(data);
    
            document.getElementById("screen").setAttribute("src", url);
        }
    };

    urlbar.addEventListener('keydown', (event) => {
        if (event.key == 'Enter') {
            ws.send(JSON.stringify({
                action: 'navigate',
                url: urlbar.value,
            }));
        }
    })

    document.addEventListener('keydown', (event) => {
        if (event.target !== document.body) {
            return;
        }

        ws.send(JSON.stringify({
            action: 'key',
            key: event.key,
        }));
    });

    screen.addEventListener('click', (event) => {
        ws.send(JSON.stringify({
            action: 'click',
            x: event.offsetX,
            y: event.offsetY,
        }));
    })
}

main();
