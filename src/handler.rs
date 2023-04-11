use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Mutex, MutexGuard};
use std::thread::sleep;
use std::time::Duration;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

use crate::news::News;

pub struct Handler {
    poll_period: u64,
    poll_count: u64,
    channel_ids: Mutex<HashSet<u64>>
}

impl Handler{
    pub fn new(poll_period: u64, poll_count: u64) -> Handler {
        let handler = Handler {
            poll_period,
            poll_count,
            channel_ids: Mutex::new(HashSet::new())
        };
        println!("Reading channels.txt");
        if let Ok(file) = File::open("channels.txt"){
            for line in BufReader::new(file).lines(){
                if let Ok(parsed_line) = line{
                    if let Ok(parsed_id) = parsed_line.parse::<u64>(){
                        handler.add_channel(parsed_id);
                    }
                }
            }
        }
        print!("Channels:");
        let channels = handler.get_channels();
        for channel in channels.iter(){
            print!(" {channel}");
        }
        println!();
        handler
    }

    pub fn get_channels(&self) -> HashSet<u64> {
        self.channel_ids.lock().unwrap().clone()
    }

    fn write_channels_to_file(map: MutexGuard<HashSet<u64>>){
        let mut file = File::create("channels.txt").expect("Couldn't open channels.txt");
        for id in map.iter(){
            writeln!(file, "{id}").expect("Couldn't write to channels.txt");
        }
    }

    pub fn add_channel(&self, id: u64){
        let mut map = self.channel_ids.lock().unwrap();
        map.insert(id);
        Handler::write_channels_to_file(map);
    }

    fn remove_channel(&self, id: u64){
        let mut map = self.channel_ids.lock().unwrap();
        map.remove(&id);
        Handler::write_channels_to_file(map);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let id = msg.channel_id.0;
        match msg.content.as_str() {
            "!stobot" => {
                self.add_channel(id);
                println!("Registered channel with ID {id}");
                let mut out_str = String::from("Registered channels:");
                let registered_channels = self.get_channels();
                for channel in registered_channels.iter(){
                    out_str += format!(" {channel}").as_str();
                }
                println!("{out_str}");
                let response = "This channel will now have STO news posted.";
                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    eprintln!("Error sending message: {why}");
                }
            }
            "!unstobot" => {
                self.remove_channel(id);
                println!("Removed channel with ID {id}");
                let mut out_str = String::from("Registered channels:");
                let registered_channels = self.get_channels();
                for channel in registered_channels.iter(){
                    out_str += format!(" {channel}").as_str();
                }
                println!("{out_str}");
                let response = "This channel will no longer have STO news posted.";
                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    eprintln!("Error sending message: {why}");
                }
            }
            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, _: Ready) {
        let mut old_news = News::new();
        if let Some(news) = News::get_news_from_json(self.poll_count).await {
            old_news = news;
        }
        loop {
            sleep(Duration::from_secs(self.poll_period));
            if let Some(news) = News::get_news_from_json(self.poll_count).await {
                let diff = news.get_different_items(&old_news);
                for item in diff {
                    for channel_id in self.get_channels().iter(){
                        let channel_id = *channel_id;
                        let channel = ChannelId(channel_id);
                        println!("Sending news with ID {} to channel with ID {}", item.id, channel_id);
                        if let Err(why) = channel.say(&ctx.http, item.to_string().as_str()).await {
                            eprintln!("Error sending message: {why}");
                        }
                    }
                }
                old_news = news;
            }
        }
    }
}
