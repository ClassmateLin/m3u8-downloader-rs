use std::{collections::HashMap, path::Path, time::Duration, thread};
use pbr::{Units, ProgressBar};
use tokio::{fs, io::{AsyncWriteExt}};
use url::{Url};
use clap::Parser;

const M3U8_EXT_HEADER: &str = "#EXTM3U";
const M3U8_EXT_INF: &str = "#EXTINF";
const M3U8_EXT_ENDLIST: &str = "#EXT-X-ENDLIST";
const M3U8_EXT_KEY: &str = "#EXT-X-KEY:";
const M3U8_EXT_KEY_METHOD: &str = "METHOD=";
const M3U8_EXT_KEY_URI: &str = "URI=";
const M3U8_EXT_KEY_IV:&str = "IV=";

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, default_value_t=String::from("http://localhost:8000/playlist.m3u8"))]
    url: String,
}

async fn get_m3u8_content(url:String) -> Result<String, Box<dyn std::error::Error>> {
    let resp = reqwest::get(url).await?.text().await?;
    Ok(resp.to_string())
}

// validate m3u8 content
fn validate_m3u8_content(text:String) -> bool {
    text.starts_with(M3U8_EXT_HEADER)
}

/// get ts list
fn get_ts_list(link:String, text:String) -> Result<Vec::<String>,  Box<dyn std::error::Error>> {
    let mut v = Vec::<String>::new();
    let mut flag = false;
    let url = Url::parse(link.as_str())?;
    for item in text.lines() {
        if item.starts_with(M3U8_EXT_INF) {
            flag = true;
            continue;
        }

        if flag {
            let ts = url.join(item.clone())?.to_string();
            v.push(ts);
            flag = false;
        }

        if item.starts_with(M3U8_EXT_ENDLIST) {
            break;
        }
    }
    Ok(v)
}

// get key info
fn get_key_info(content: String) -> HashMap<String, String> {
    
    let mut m = HashMap::<String, String>::new();
    
    for item in content.lines() {
        if !item.starts_with(M3U8_EXT_KEY){
            continue;
        }
        let data = item.replace(M3U8_EXT_KEY, "").replace('"', "");
        for it in data.split(",") {
            
            if it.starts_with(M3U8_EXT_KEY_METHOD){
                let tmp = it.to_string().replace(M3U8_EXT_KEY_METHOD, "");
                m.insert("method".to_string(), tmp);
            }
            
            if it.starts_with(M3U8_EXT_KEY_IV){
                let tmp = it.to_string().replace(M3U8_EXT_KEY_IV, "");
                m.insert("iv".to_string(), tmp);
            }

            if it.starts_with(M3U8_EXT_KEY_URI){
                let tmp = it.to_string().replace(M3U8_EXT_KEY_URI, "");
                m.insert("uri".to_string(), tmp);
            }
            
        }
    }
    m
}

// get key content
async fn get_key_content(url: String) ->Result<String, Box<dyn std::error::Error>> {
    let text = reqwest::get(&url).await?.text().await?;
    Ok("key".to_string())
}

// download file
async fn download(link: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
    .build()?;

    let dirname = "download".to_string();
    if !Path::new(&dirname).exists(){
        fs::create_dir(&dirname).await?;
    }
    
    let url = Url::parse(&link)?;

    let respone = client.get(&link).send().await?;
    let content_length = respone.content_length().unwrap(); 
    
    println!("content length: {}", content_length);
    let mut pb = ProgressBar::new(content_length);
    pb.set_units(Units::Bytes);
    
    let filename = String::from("./download") + &url.path().to_string();
  
    let mut source = client.get(&link).send().await?;
 
    let mut dest = fs::OpenOptions::new().create(true).append(true).open(&filename).await?;
    
    while let Some(chunk) = source.chunk().await? {
        dest.write_all(&chunk).await?;
        pb.add(chunk.len() as u64);
        thread::sleep(Duration::from_millis(10));
    }
    pb.finish_println(format!("下载成功, {}", filename).as_str());
    Ok(())
}

// download all
async fn download_all(ts_list: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("总共{}个ts文件...", ts_list.len());

    for item in ts_list.iter() {
        let link = item.to_string().clone();
        download(link).await?;

    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let content = match get_m3u8_content(args.url.clone()).await {
        Ok(text) => text,
        Err(error) => panic!("Can not get m3u8 info: {:?}", error),
    };


    if !validate_m3u8_content(content.clone()){
        println!("错误的m3u8...");
    }else {
        let ts_list = match get_ts_list(args.url.clone(), content.clone()) {
            Ok(ts) => ts,
            Err(_) => vec![],
        };
        let key_info = get_key_info(content.clone());
        let method = match key_info.get("method") {
            Some(method) => method.to_string(),
            None => "".to_string(),
        };
        if method == "" {
            download_all(ts_list).await?;
        }else {
            let _iv = match key_info.get("iv") {
                Some(method) => method.to_string(),
                None => "".to_string(),
            };
            let _key = match key_info.get("uri") {
                Some(key) => get_key_content(key.clone()).await?,
                None => "".to_string(),
            };
            download_all(ts_list).await?;
        }
        
    }
    
    Ok(())
}
