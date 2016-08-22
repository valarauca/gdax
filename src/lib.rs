
extern crate openssl;
use openssl::ssl::{SslMethod,SslContext};

extern crate websocket;
use websocket::Client;
use websocket::dataframe::{DataFrame,Opcode};
use websocket::stream::WebSocketStream;
use websocket::sender::Sender;
use websocket::receiver::Receiver;
use websocket::client::request::Url;

#[macro_use]
extern crate lazy_static;

extern crate regex;
use regex::Regex;

use std::mem;

//
//constant strings values
//
const WEBSOCKET: &'static str =
    "wss://ws-feed.gdax.com";
const SUBSCRIBE: &'static str =
    r#"{"type":"subscribe","product_id":"BTC-USD"}"#;
const HEARTBEAT: &'static str =
    r#"{"type":"heartbeat","on":true}"#;

//
//Errors that happen while connecting
//
#[derive(Debug)]
pub enum Conn {
    OpenSSL(::openssl::ssl::error::SslError),
    Connection(::websocket::result::WebSocketError),
    General
}

//
//Function to connect to GDAX
//
pub fn connect_gdax()
-> Result<
            Client<DataFrame,Sender<WebSocketStream>,Receiver<WebSocketStream>>,
            Conn>
{
    let sslcontext = match SslContext::new( SslMethod::Sslv23 ) {
        Ok(x) => x,
        Err(e) => return Err(Conn::OpenSSL(e))
    };
    let url = Url::parse( WEBSOCKET ).unwrap();
    let req = match Client::connect_ssl_context(url, &sslcontext) {
        Ok(x) => x,
        Err(e) => return Err(Conn::Connection(e))
    };
    let res = match req.send() {
        Ok(x) => x,
        Err(e) => return Err(Conn::Connection(e))
    };
    match res.validate() {
        Ok(_) => { },
        Err(e) => return Err(Conn::Connection(e))
    };
    let heartbeat = DataFrame {
        finished: true,
        reserved: [false,false,false],
        opcode: Opcode::Text,
        data: HEARTBEAT.to_string().into_bytes()
    };
    let subscribe = DataFrame {
        finished: true,
        reserved: [false,false,false],
        opcode: Opcode::Text,
        data: SUBSCRIBE.to_string().into_bytes()
    };
    let mut client = res.begin();
    match client.send_dataframe( &heartbeat ) {
        Ok(_) => { },
        Err(e) => return Err(Conn::Connection(e))
    };
    match client.send_dataframe( &subscribe ) {
        Ok(_) => { },
        Err(e) => return Err(Conn::Connection(e))
    };
    Ok(client)
}

//
//Borrow text as a read only pointer from a data frame
//
pub fn dataframe_text<'a>( df: &'a DataFrame ) -> Option<&'a str> {
    match df.opcode {
        Opcode::Text => { },
        _ => return None
    };
   Some( unsafe{ mem::transmute( df.data.as_slice() ) } )
}

//
//Define a set of regex's to handle parsing incoming packets
//
lazy_static! {
    
    static ref TYPECODE: Regex = Regex::new(
    r#"[\s\S]*"type":\s*"(\w+)"[\s\S]*"#).unwrap();
    
    static ref TIME: Regex = Regex::new(
    r#"[\s\S]*"time":\s*"(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d).(\d{6})[\s\S]*"#).unwrap();

    static ref TYPE: Regex = Regex::new(
    r#"[\s\S]*"type":\s*"(\w+)"[\s\S]*"#).unwrap();
    
    static ref SIDE: Regex = Regex::new(
    r#"[\s\S]*"side"\s*"(\w+)"[\s\S]*"#).unwrap();

    static ref ORDER_ID: Regex = Regex::new(
    r#"[\s\S]*"order_id":\s*"([a-f\d]{8})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{12})"[\s\S]*"#).unwrap();

    static ref MORDER_ID: Regex = Regex::new(
    r#"[\s\S]*"maker_order_id":\s*"([a-f\d]{8})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{12})"[\s\S]*"#).unwrap();

    static ref TORDER_ID: Regex = Regex::new(
    r#"[\s\S]*"taker_order_id":\s*"([a-f\d]{8})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{4})-([a-f\d]{12})"[\s\S]*"#).unwrap();

}

//
//Date structure for tracking time
//
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
#[repr(C)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    min: u8,
    sec: u8,
    ms: u32
}impl Date {

    //
    //Parse a date from a packet
    //
    pub fn new( buffer: &str ) -> Option<Date> {
        let caps = match TIME.captures(buffer) {
            Option::Some(x) => x,
            Option::None => return None
        };
        let mut ret_val: Date = unsafe{ mem::zeroed() };
        ret_val.year = match u16::from_str_radix(caps.at(1).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.month = match u8::from_str_radix(caps.at(2).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.day = match u8::from_str_radix(caps.at(3).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.hour = match u8::from_str_radix(caps.at(4).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.min = match u8::from_str_radix(caps.at(5).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.sec = match u8::from_str_radix(caps.at(6).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        ret_val.ms = match u32::from_str_radix(caps.at(7).unwrap(),10){
            Ok(x) => x,
            Err(_) => return None
        };
        Some(ret_val)
    }
    //
    //Parse return to String
    //
    pub fn to_str(&self) -> String {
        format!(
"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",self.year,self.month,self.day,self.hour,self.min,self.sec,self.ms)
    }
}

#[test]
fn test_date_conversion() {
    let test_packet: &'static str = r#"
{
    "type": "received",
    "time": "2014-11-07T08:19:27.028459Z",
    "product_id": "BTC-USD",
    "sequence": 10,
    "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
    "size": "1.34",
    "price": "502.1",
    "side": "buy",
    "order_type": "limit"
}"#;

    let date = Date::new( test_packet ).unwrap();
    let s = date.to_str();
    assert_eq!(s, "2014-11-07T08:19:27.028459Z".to_string());
}

//
//Order ID prototype
//

#[derive(Debug,Clone,Copy)]
pub enum OrderKind {
    Normal,
    Maker,
    Taker
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
#[repr(C)]
pub struct OrderID {
    x: u64,
    y: u64
}
impl OrderID {
    pub fn new( buffer: &str, kind: OrderKind ) -> Option<OrderID> {
        let caps = match kind {
            OrderKind::Normal => match ORDER_ID.captures(buffer){
                Option::Some(x) => x,
                Option::None => return None
            },
            OrderKind::Maker => match MORDER_ID.captures(buffer){
                Option::Some(x) => x,
                Option::None => return None
            },
            OrderKind::Taker => match TORDER_ID.captures(buffer){
                Option::Some(x) => x,
                Option::None => return None
            }
        };
        let mut id: OrderID = unsafe{ mem::zeroed() };
        id.x += match u64::from_str_radix(caps.at(1).unwrap(),16){
            Ok(x) => x,
            Err(_) => return None
        };
        id.x << 4;
        id.x += match u64::from_str_radix(caps.at(2).unwrap(),16){
            Ok(x) => x,
            Err(_) => return None
        };
        id.x << 2;
        id.x += match u64::from_str_radix(caps.at(3).unwrap(),16){
            Ok(x) => x,
            Err(_) => return None
        };
        id.y += match u64::from_str_radix(caps.at(4).unwrap(),16){
            Ok(x) => x,
            Err(_) => return None
        };
        id.y << 2;
        id.y += match u64::from_str_radix(caps.at(5).unwrap(),16){
            Ok(x) => x,
            Err(_) => return None
        };
        Some(id)
    }
}
