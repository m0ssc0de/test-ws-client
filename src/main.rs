//! Simple websocket client.
use std::time::Duration;
use std::{io, thread};

use actix::io::SinkWrite;
use actix::*;
use actix_codec::Framed;
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket, Client,
};
use bytes::Bytes;
use futures::stream::{SplitSink, StreamExt};
mod pmodels;

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let sys = System::new("websocket-client");

    Arbiter::spawn(async {
        let (response, framed) = Client::new()
            .ws("http://127.0.0.1:8088/medicine/ws/")
            .connect()
            .await
            .map_err(|e| {
                println!("Error: {}", e);
            })
            .unwrap();

        println!("{:?}", response);
        let (sink, stream) = framed.split();
        let addr = ChatClient::create(|ctx| {
            ChatClient::add_stream(stream, ctx);
            ChatClient(SinkWrite::new(sink, ctx))
        });

        // start console loop
        thread::spawn(move || loop {
            let mut cmd = String::new();
            if io::stdin().read_line(&mut cmd).is_err() {
                println!("error");
                return;
            }
            addr.do_send(ClientCommand(cmd));
        });
    });
    sys.run().unwrap();
}

struct ChatClient(SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>);

#[derive(Message)]
#[rtype(result = "()")]
struct ClientCommand(String);

impl Actor for ChatClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // start heartbeats otherwise server will disconnect after 10 seconds
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("Disconnected");

        // Stop application on disconnect
        System::current().stop();
    }
}

impl ChatClient {
    fn hb(&self, ctx: &mut Context<Self>) {
        // let s = pmodels::create_large_shirt(String::from("man in black"));
        // let a = pmodels::serialize_shirt(&s);
        let mut cm = pmodels::create_query();
        let a = pmodels::s_client_message(&cm);
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.0
                // .write(Message::Ping(Bytes::from_static(b"")))
                // .write(Message::Text(String::from("3q")))
                .write(Message::Binary(a.into()))
                .unwrap();
            act.hb(ctx);

            // client should also check for a timeout here, similar to the
            // server code
        });
    }
}

/// Handle stdin commands
impl Handler<ClientCommand> for ChatClient {
    type Result = ();

    fn handle(&mut self, msg: ClientCommand, _ctx: &mut Context<Self>) {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for ChatClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(txt)) => println!("Server: {:?}", txt),
            Ok(Frame::Binary(bin)) => {
                let m = pmodels::d_client_message(&bin);
                match m {
                    Ok(m) => {
                        println!("Ok(m) m: {:?}", m);
                        match m.message {
                            Some(msg) => {
                                println!("msg {:?}", msg);
                            }
                            None => {
                                println!("got none");
                            }
                        }
                    }
                    Err(e) => println!("got err {}", e),
                }
            }
            _ => println!("Nothing"),
        }
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("Connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        println!("Server disconnected");
        ctx.stop()
    }
}

impl actix::io::WriteHandler<WsProtocolError> for ChatClient {}
