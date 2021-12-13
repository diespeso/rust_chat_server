use std::collections::HashMap;

use actix::{Actor, ActorFuture,
    ContextFutureSpawner, ActorContext,
    WrapFuture, StreamHandler,
    Handler, Addr,
    AsyncContext, Message,
    Recipient, Context,
    fut, fut::FutureWrap, prelude::Future};

use actix_web_actors::ws;

use serde::{Serialize};
use serde_json;

//messages
#[derive(Message, Serialize)]
#[rtype(result = "()")]
/// WebSocket Message, A Message to a WebSocket in the client side(see public/script/chat.js)
pub struct WSockMess {
    pub msg: String,
    pub sender: String, //id
}

impl WSockMess {
    pub fn new(msg: String, sender: String) -> Self {
        Self {
            msg, sender
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
/// Connection Message, the first message a client websocket sends
pub struct ConnMess{ 
    /// The recipient of a web socket message, in this case the web socket on the client side.
    pub addr_res: Recipient<WSockMess>,
    /// ID (name) of the user connecting
    pub id: String,
}

#[derive(Message)]
#[rtype(result="()")]
/// Disconnection Message, notifies the feed that a user is going to disconect.
pub struct DiscMess {
    pub id: String,
}

#[derive(Message)]
#[rtype(result="()")]
/// Chat Socket Message, a message for users to communicate with the feed.
pub struct ChatSockMess {
    /// Id of the user sending
    pub id: String,
    /// Message sent
    pub msg: String,
}

#[derive(Debug)]
/// Chat Socket, Server side representation of a user in the chat (feed)
pub struct ChatSock {
    /// Address of the chat the user is using
    pub feed_addr: Addr<Feed>,
    /// ID the user is using in the feed
    pub id: String,
}

impl ChatSock {
    pub fn new(id: String, feed: Addr<Feed>) -> Self {
        Self {
            feed_addr: feed,
            id
        }
    }
}

/// Control Actor to check if a id is already in use
pub struct CheckId {
    pub id: String,
}

impl Message for CheckId {
    type Result = bool;
}

impl Actor for ChatSock {
    type Context = ws::WebsocketContext<Self>; // Using websockets

    fn started(&mut self, ctx: &mut Self::Context) { // creating a new user in the chat
        println!("started chatsock: {}", self.id);
        let addr_ws = ctx.address(); //get address of this chatsock
        if let Err(err) = self.feed_addr
            .try_send(ConnMess { /// try to send a connection message to register in the chat
                addr_res: addr_ws.recipient(), //this chatsock
                id: self.id.clone()
            }) {
                println!("error al enviar: {}", err);
            }
            /*.into_actor(self) //with normal send, no try_send
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);*/
            
            
    }
}

impl Handler<WSockMess> for ChatSock {
    type Result = ();

    /// A Chatsock will receive the message inside a WSockMess and send it to the client side websocket
    fn handle(&mut self, msg: WSockMess, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&msg).expect("Failed to make json from WSockMessage on ChatSock")); //from chatsocket in the server side to websocket in the client side
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSock {

    /// handling of websocket messages from the clients web socket
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => { // normal websocket message
                println!("client {:?}: {:?}", ctx.address(), txt);
                self.feed_addr.do_send(ChatSockMess{  //send a message from this chatsock to the feed
                    id: self.id.clone(),
                    msg: txt.clone()
                });
            },
            Ok(ws::Message::Ping(data)) => { //not really used
                println!("ping! {:?}: {:?}", ctx.address(), &data);
                ctx.pong(&data);
            },
            Ok(ws::Message::Pong(_)) => { //not really used
                println!("pong: {:?}", ctx.address());
            },
            Ok(ws::Message::Binary(bin)) => { //might be useful for images or idk, im not sure, not used yet
                println!("bin {:?}: {:?}", ctx.address(), bin);
            },
            Ok(ws::Message::Close(reason)) => { // message notifying that the websocket is closed
                println!("close {:?}: {:?}", ctx.address(), reason);
                //send a message to delete this chatsock from feed using its id
                self.feed_addr.do_send(DiscMess{id: self.id.clone()}); 
                println!("forgotten user {} from feed", self.id.clone());
                ctx.close(reason); //close the chatsock.
            }
            Err(err) => {
                panic!("{}", err);
            },
            _ => {println!("didnt handle the wsmessage in streamhandler")}
        }
    }
}

#[derive(Debug)]
/// The Feed is the handler of all users (clients) in the chat
pub struct Feed {
    /// Relation of ID: recipent that can handle WSockMess
    pub clients: HashMap<String, Recipient<WSockMess>>,

}

impl Feed {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }
}

impl Actor for Feed {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        //just debug print, here the actor for feed starts
        println!("Feed is up and running: {:?}", ctx.address());
    }
}

impl Handler<ConnMess> for Feed {
    type Result = ();

    /// add user to this feeds clients on connection message
    fn handle(&mut self, msg: ConnMess, ctx: &mut Self::Context) {
        println!("Received connection, connecting {} to server...", msg.id);
        self.clients.insert(msg.id, msg.addr_res);
    }
}

impl Handler<CheckId> for Feed {
    type Result = bool; //true if the user is taken, false otherwise

    fn handle(&mut self, msg: CheckId, ctx: &mut Self::Context) -> Self::Result {
        println!("{:?}, {:?}: {:?}", &msg.id, self.clients, self.clients.contains_key(&msg.id));
        self.clients.contains_key(&msg.id)
    }
}

impl Handler<DiscMess> for Feed {
    type Result = ();

    /// If the user disconnected, erase its chatsocket on this feed
    fn handle(&mut self, msg: DiscMess, ctx: &mut Self::Context) {
        self.clients.remove(&msg.id);
    }
}

impl Handler<ChatSockMess> for Feed {
    type Result = ();

    // how to handle a client message sent to the feed
    fn handle(&mut self, msg: ChatSockMess, ctx: &mut Self::Context) {
        /*let mut texto = &msg.msg.split(':').map( //TODO:BUG: This makes it imposible to send emoticons like: :c :) or any other
            |s| {
                s.to_owned()
            }
        ).collect::<Vec<String>>(); //split: before :, and after : is the message contents*/
        if msg.msg.clone().len() as i32 > 1 { //nonempty message split
            //send the message received to all other chatsockets in the feed 
            for (_, v) in &mut self.clients {
                if let Err(err) = v.do_send(
                    WSockMess::new(msg.msg.clone(), msg.id.clone())) { //wsockmessage fort the chatsock to handle
                    println!("Couldnt send ChatSockMess to all participants: {}", err);
                }
            }
        }
        println!("[{} SAYS: {}]", msg.id, msg.msg); //debug
    }
}