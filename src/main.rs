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
        .args(&["pull", "--rebase"])
        .spawn()
        .expect("failed to execute child");
    let _output = child.wait_with_output().expect("failed to wait on child");
    //println!("{:?}", _output);
}

pub fn git_sync(dir: &str) {
    let child = Command::new("git")
        .current_dir(dir)
        .args(&["add", "."])
        .spawn()
        .expect("failed to execute child");
    let _output = child.wait_with_output().expect("failed to wait on child");
    //println!("{:?}", output);

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
    let _output = child.wait_with_output().expect("failed to wait on child");
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
            let vec: Vec<&str> = l.split(" ").collect();
            let elems: Vec<String> = vec
                .iter()
                .filter(|&x| x.trim().len() > 0)
                .map(|f| f.trim().to_string())
                .collect();
            if elems.len() > 1 {
                return elems[1..].join(" ").trim().to_string();
            }
        }
    }
    return "".to_string();
}

fn process_images(lines: &[&str], hexo_target: &str, files: &mut Vec<(String, String)>) -> String {
    let mut res = vec![];
    for line in lines {
        let s = line.trim();
        let mut f = "".to_string();
        let mut alt = "".to_string();
        let mut new_file_name = "".to_string();
        if s.starts_with("![[") && s.ends_with("]]") {
            let file = s.replace("![[", "").replace("]]", "");
            if file.contains("|") {
                let v: Vec<&str> = file.split("|").collect();
                f = v[0].trim().to_string();
                alt = v[1].trim().to_string();
            } else {
                f = file.clone();
            }
            new_file_name = format!("/images/ob_{}", f.replace(" ", "-"));
        } else if s.starts_with("![") && s.ends_with(")") {
            let pos = s.find("(");
            if pos.is_some() {
                let t = s[pos.unwrap() + 1..s.len() - 1].trim().to_string();
                f = t
                    .split("/")
                    .collect::<Vec<&str>>()
                    .last()
                    .unwrap_or(&"")
                    .to_string();
                new_file_name = format!("/images/ob_{}", f.replace(" ", "-"));
            }
        }
        if f != "" && new_file_name != "" {
            let img = format!("./Pics/{}", f);
            let image_name = new_file_name.to_lowercase();
            let target = format!("{}/source{}", hexo_target, image_name);
            //println!("img: {} => {}", img, target);
            files.push((img, target));
            let l = format!("![{}]({})", &alt, &image_name);
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
        //println!("entry: {:?}", entry);
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
        //println!("link: {:?}\nsite: {:?}\ntitle: {:?}\n", link, site, title);
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
        let strip_tags = tags.replace("，", ",");
        let tags_elems = strip_tags.split(",").collect::<Vec<&str>>();
        let mut tags_str = "".to_string();
        for t in tags_elems {
            tags_str.push_str(&format!("- {}\n", t.trim()));
        }
        let hexo_meta = format!(
            "---\nlayout: post\ntitle: '{}'\ndate: {}\ntags: \n{}\n",
            title, time_str, tags_str
        );
        //println!("hexo_meta: {}", hexo_meta);

        let mut files = vec![];
        let hexo_body = process_images(body, &hexo_target, &mut files);
        let content = format!("{}\n---\n{}", hexo_meta, hexo_body);

        if prev_content == content {
            continue;
            //println!("path: {}", path);
            //println!("no change: {:?}", path);
        } else {
            for file in files {
                let (src, dst) = file;
                println!("copy: {:?} => {:?}", src, dst);
                fs::copy(src, dst).unwrap();
            }
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
        .arg("-p, --publish  'Remove all the pages for a feed'")
        .arg("-n, --name=[NAME] 'Site name'")
        .arg("-t, --target=[TARGET] 'Site directory'")
        .get_matches();

    let mut conf = Conf {
        sites: HashMap::new(),
    };

    let name = matches.value_of("name").unwrap_or("");
    let target = matches.value_of("target").unwrap_or("");
    if name != "" && target != "" {
        conf.sites.insert(name.to_string(), target.to_string());
    }

    let default_conf = "./Pub/config.md";
    let conf_file = fs::read_to_string(Path::new(default_conf)).unwrap_or_default();
    for line in conf_file.lines() {
        let elems = line.split(":").collect::<Vec<&str>>();
        if elems.len() != 2 {
            continue;
        }
        let from = elems[0].trim();
        let target = elems[1].trim();
        conf.sites.insert(from.to_string(), target.to_string());
    }

    if conf.sites.is_empty() {
        println!("No site configured");
        return;
    }

    git_pull("./");
    for (k, v) in &conf.sites {
        println!("update site {} at {}:", k, v);
        git_pull(v);
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

    #[test]
    fn test_try_title() {
        let lines = vec!["title: hello", "### title world"];
        assert_eq!(try_title(&lines), "title world");

        let lines = vec!["title: hello", "### "];
        assert_eq!(try_title(&lines), "");

        let lines = vec!["title: hello", "### ", "# This good way"];
        assert_eq!(try_title(&lines), "This good way");

        let lines = vec!["title: hello", "### 你好，世界"];
        assert_eq!(try_title(&lines), "你好，世界");
    }

    #[test]
    fn test_images() {
        let lines = vec![
            "![](/Pics/2021-11-29-16-37-14-demo1.png)",
            "![](/Pics/2021-11-29-16-37-14-demo2.png)",
        ];
        let mut files1 = vec![];
        let res1 = process_images(&lines, "./blog/directory", &mut files1);
        assert_eq!(files1.len(), 2);
        /* println!("{:?}", files1);
        println!("{}", res1); */

        let lines = vec![
            "![[2021-11-29-16-37-14-demo1.png]]",
            "![[2021-11-29-16-37-14-demo2.png]]",
        ];
        let mut files2 = vec![];
        let res2 = process_images(&lines, "./blog/directory", &mut files2);
        assert_eq!(files2.len(), 2);
        /*   println!("{:?}", files2);
        println!("{}", res2); */

        assert_eq!(files1, files2);
        assert_eq!(res1, res2);
    }
}
