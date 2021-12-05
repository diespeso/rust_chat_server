let socket;

window.onload = function() {
    let user = document.getElementById("name").textContent;

    socket = new WebSocket("ws://192.168.1.64:8080/ws/init" + "/" + user);
    console.log(socket);

    socket.onopen = function() {
        socket.send("bonjour");
    };

    socket.onmessage = function(msg) {
        console.log("server: " + msg.data); //debug
        let instr = msg.data.split(":");
        if(instr[0] == "all") {
            //add to general feed
            document.getElementById("feed_view").innerHTML = document.getElementById("feed_view").innerHTML + gen_message_card(instr[1], instr[2]); 
        }
    }
}

function send_message() {
    str_msg = "0:" + document.getElementsByName("message")[0].value;
    console.log(str_msg);
    socket.send(str_msg);
}

function gen_message_card(id, message) {
    let html = `
    <div class="message_card">
        <p><span class="mes_id">${id}</span>:${message}</p>
    </div>
    `;
    return html;
}