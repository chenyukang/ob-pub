use std::{collections::HashMap, fs};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use clap::App;
use glob::glob;
use std::path::Path;

#[derive(Debug)]
struct Conf {
    pub sites: HashMap<String, String>,
}

use std::process::Command;

pub fn git_pull(dir: &str) {
    let child = Command::new("git")
        .current_dir(dir)
        .args(&["pull", "origin", "main", "--rebase"])
        .spawn()
        .expect("failed to execute child");
    let output = child.wait_with_output().expect("failed to wait on child");
    println!("{:?}", output);
}

pub fn git_sync(dir: &str) {
    let child = Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .spawn()
        .expect("failed to execute child");
    let output = child.wait_with_output().expect("failed to wait on child");
    println!("{:?}", output);

    let child = Command::new("git")
        .current_dir(dir)
        .args(&["commit", "-am'ob-web'"])
        .spawn()
        .expect("failed to execute child");
    let output = child.wait_with_output().expect("failed to wait on child");
    println!("{:?}", output);

    git_pull(dir);

    let child = Command::new("git")
        .current_dir(dir)
        .args(&["push"])
        .spawn()
        .expect("failed to execute child");
    let output = child.wait_with_output().expect("failed to wait on child");
    println!("{:?}", output);
}

fn publish(conf: &Conf) {
    for (site, url) in &conf.sites {
        println!("publish {} to {}", site, url);
        git_sync(url);
    }
}

fn try_key(lines: &[&str], key: &str) -> String {
    for line in lines {
        let pos = line.find(":");
        if pos.is_none() {
            continue;
        }
        let k = &line[0..pos.unwrap()].trim();
        let v = &line[pos.unwrap() + 1..].trim();
        if *k == key {
            return v.to_string();
        }
    }
    return "".to_string();
}

fn try_site(sites: &HashMap<String, String>, lines: &[&str]) -> String {
    for (k, _) in sites {
        for l in lines {
            if l.contains(&format!("[[{}]]", k)) {
                return k.clone();
            }
        }
    }
    return "".to_string();
}

fn try_title(lines: &[&str]) -> String {
    for l in lines {
        if l.trim().starts_with("#") {
            return l.replace("#", "").trim().to_string();
        }
    }
    return "".to_string();
}

fn process_images(lines: &[&str], hexo_target: &str) -> String {
    let mut res = vec![];
    for line in lines {
        let s = line.trim();
        if s.starts_with("![[") && s.ends_with("]]") {
            let f = s.replace("![[", "").replace("]]", "");
            let img = format!("./Pics/{}", f);
            let new_file_name = format!("/images/ob_{}", f.replace(" ", "_"));
            let target = format!("{}/source{}", hexo_target, new_file_name);
            println!("img: {} => {}", img, target);
            fs::copy(img, target).unwrap();
            let l = format!("![{}]({})", &new_file_name, &new_file_name);
            res.push(l);
        } else {
            res.push(line.to_string());
        }
    }
    res.join("\n")
}

fn sync_posts(conf: &Conf) {
    println!("conf: {:?}", conf);
    let mut files = vec![];
    for entry in glob("./Pub/**/*.md").expect("failed") {
        match entry {
            Ok(path) => {
                files.push(format!("{}", path.display()));
            }
            Err(e) => println!("{:?}", e),
        }
    }
    for file in files {
        let content = fs::read_to_string(Path::new(&file)).expect("failed to read file");
        let lines = content.lines().collect::<Vec<&str>>();
        let index = lines
            .iter()
            .position(|&x| x.starts_with("---"))
            .unwrap_or(usize::MAX);
        if index == usize::MAX {
            continue;
        }
        let body = &lines[index + 1..];
        let meta = &lines[..index];
        let link = try_key(meta, "pub_link");
        let tags = try_key(meta, "pub_tags");
        let site = try_site(&conf.sites, meta);
        let title = try_title(meta);
        println!("link: {:?}\nsite: {:?}\ntitle: {:?}\n", link, site, title);
        if title == "" || site == "" || link == "" {
            continue;
        }
        //println!("publish: {:?}", file);

        let hexo_target = &conf.sites[&site];
        let path = format!("{}/source/_posts/{}.md", &hexo_target, link);

        let time: DateTime<Tz> = Utc::now().with_timezone(&Tz::Asia__Chongqing);
        let mut time_str = time.format("%Y-%m-%d %H:%M:%S").to_string();

        let prev_content = fs::read_to_string(Path::new(&path)).unwrap_or(String::default());
        if prev_content != "" {
            let lines = prev_content.lines().collect::<Vec<&str>>();
            let prev_time = try_key(&lines, "date");
            //println!("prev_time: {:?}", prev_time);
            time_str = prev_time.clone();
        }
        let hexo_meta = format!(
            "---\nlayout: post\ntitle: {}\ndate: {}\ntags: [{}]\n",
            title, time_str, tags
        );
        //println!("hexo_meta: {}", hexo_meta);

        let hexo_body = process_images(body, &hexo_target);
        let content = format!("{}\n---\n{}", hexo_meta, hexo_body);

        println!("path: {}", path);
        if prev_content == content {
            println!("no change: {:?}", path);
        } else {
            println!("publish: {:?}", path);
            fs::write(path, content).unwrap();
        }
    }
}

fn main() {
    let matches = App::new("Ob-pub")
        .version("0.1")
        .author("yukang <moorekang@gmail.com>")
        .about("Publish Obsidian to Hexo")
        .arg("-s, --sync     'Sync posts in Obsidian into Hexo'")
        .arg("-p, --publish    'Remove all the pages for a feed'")
        .get_matches();

    let mut conf = Conf {
        sites: HashMap::new(),
    };

    let conf_file = fs::read_to_string(Path::new("./Pub/config.md")).unwrap();
    for line in conf_file.lines() {
        let elems = line.split(":").collect::<Vec<&str>>();
        if elems.len() != 2 {
            continue;
        }
        let key = elems[0].trim();
        let value = elems[1].trim();
        conf.sites.insert(key.to_string(), value.to_string());
    }

    if matches.is_present("sync") {
        sync_posts(&conf);
    }

    if matches.is_present("publish") {
        publish(&conf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_key() {
        let lines = vec!["title: hello", "date: 2021-11-23 00:21:58"];
        assert_eq!(try_key(&lines, "title"), "hello");
        assert_eq!(try_key(&lines, "date"), "2021-11-23 00:21:58");
    }
}
