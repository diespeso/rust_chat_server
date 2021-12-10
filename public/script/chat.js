let socket;

window.onload = function() {
    let user = document.getElementById("name").textContent; //server puts the username in this html element, gotta retrieve here

    socket = new WebSocket("ws://192.168.1.64:8080/ws/init" + "/" + user); //connect to a websocket in the server (see main.rs), the ip is my pcs ip
    console.log(socket);

    socket.onopen = function() { //first message just for debugging
        socket.send("bonjour");
    };

    socket.onmessage = function(msg) {
        console.log("server: " + msg.data); //debug
        let instr = msg.data.split(":"); //before : theres an specification of who should see the message, this is prob bad (sorry)
        if(instr[0] == "all") {
            //add message to general feed
            document.getElementById("feed_view").innerHTML = document.getElementById("feed_view").innerHTML + gen_message_card(instr[1], instr[2]); 
        }
    }
}

function send_message() {
    str_msg = "0:" + document.getElementsByName("message")[0].value; //get message in textbox
    console.log(str_msg);
    socket.send(str_msg);
}

function gen_message_card(id, message) {
    //html for a new message
    let html = `
    <div class="message_card">
        <p><span class="mes_id">${id}</span>:${message}</p>
    </div>
    `;
    return html;
}