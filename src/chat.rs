
use std::collections::HashMap;

use actix::{Actor, ActorFuture,
    ContextFutureSpawner, ActorContext,
    WrapFuture, StreamHandler,
    Handler, Addr,
    AsyncContext, Message,
    Recipient, Context,
    fut, fut::FutureWrap, prelude::Future};

use actix_web_actors::ws;



//messages
#[derive(Message)]
#[rtype(result = "()")]
pub struct WSockMess(pub String); //websocket message

#[derive(Message)]
#[rtype(result = "()")]
pub struct ConnMess{ //new connection message
    pub addr_res: Recipient<WSockMess>,
    pub id: String,
}

//socket
#[derive(Debug)]
pub struct ChatSock {
    pub feed_addr: Addr<Feed>,
    pub id: String,
}

//check existance
pub struct CheckId {
    pub id: String,
}

#[derive(Message)]
#[rtype(result="()")]
pub struct DiscMess {
    pub id: String,
}

#[derive(Message)]
#[rtype(result="()")]
pub struct ChatSockMess {
    pub id: String,
    pub msg: String,
}

impl Message for CheckId {
    type Result = bool;
}

impl ChatSock {
    pub fn new(id: String, feed: Addr<Feed>) -> Self {
        Self {
            feed_addr: feed,
            id
        }
    }
}

impl Actor for ChatSock {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("started chatsock: {}", self.id);
        let addr_ws = ctx.address();
        if let Err(err) = self.feed_addr
            .try_send(ConnMess {
                addr_res: addr_ws.recipient(), //websocket
                id: self.id.clone()
            }) {
                eprint!("error al enviar: {}", err);
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
    fn handle(&mut self, msg: WSockMess, ctx: &mut Self::Context) {
        ctx.text(msg.0); //from chatsocket to websocket
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSock {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                println!("client {:?}: {:?}", ctx.address(), txt);
                self.feed_addr.do_send(ChatSockMess{
                    id: self.id.clone(),
                    msg: txt.clone()
                });
                //ctx.text("ok, perra".to_owned());
                /*if let Err(err) = ctx.address().try_send(WSockMess("ok perra saludos".to_owned())) {
                    println!("Couldnt send WSockMess: {}", err);
                }*/
            },
            Ok(ws::Message::Ping(data)) => {
                println!("ping! {:?}: {:?}", ctx.address(), &data);
                ctx.pong(&data);
            },
            Ok(ws::Message::Pong(_)) => {
                println!("pong: {:?}", ctx.address());
            },
            Ok(ws::Message::Binary(bin)) => {
                println!("bin {:?}: {:?}", ctx.address(), bin);
            },
            Ok(ws::Message::Close(reason)) => {
                println!("close {:?}: {:?}", ctx.address(), reason);
                //delete from feed
                self.feed_addr.do_send(DiscMess{id: self.id.clone()});
                println!("forgotten user {} from feed", self.id.clone());
                ctx.close(reason);
            }
            Err(err) => {
                panic!(err);
            },
            _ => {println!("didnt handle the wsmessage in streamhandler")}
        }
    }
}

#[derive(Debug)]
pub struct Feed {
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
        println!("Feed is up and running: {:?}", ctx.address());
    }
}

impl Handler<ConnMess> for Feed {
    type Result = ();
    fn handle(&mut self, msg: ConnMess, ctx: &mut Self::Context) {
        println!("Received connection, connecting {} to server...", msg.id);
        self.clients.insert(msg.id, msg.addr_res);
    }
}

impl Handler<CheckId> for Feed {
    type Result = bool;
    fn handle(&mut self, msg: CheckId, ctx: &mut Self::Context) -> Self::Result {
        println!("{:?}, {:?}: {:?}", &msg.id, self.clients, self.clients.contains_key(&msg.id));
        self.clients.contains_key(&msg.id)
    }
}

impl Handler<DiscMess> for Feed {
    type Result = ();
    fn handle(&mut self, msg: DiscMess, ctx: &mut Self::Context) {
        self.clients.remove(&msg.id);
    }
}

impl Handler<ChatSockMess> for Feed {
    type Result = ();
    fn handle(&mut self, msg: ChatSockMess, ctx: &mut Self::Context) {
        let mut texto = &msg.msg.split(':').map( //TODO:BUG: USAR ESTO HACE QUE NO SEA POSIBLE PONER CARITAS
            |s| {
                s.to_owned()
            }
        ).collect::<Vec<String>>();
        if texto.len() > 1 { //nonempty message split
            //to all
            for (_, v) in &mut self.clients {
                if let Err(err) = v.do_send(
                    WSockMess("all:".to_owned() + &msg.id.clone() + ":" + &texto[1].clone())) {
                    println!("Couldnt send ChatSockMess to all participants: {}", err);
                }
            }
        }
        println!("[{} SAYS: {}]", msg.id, msg.msg);
    }
}