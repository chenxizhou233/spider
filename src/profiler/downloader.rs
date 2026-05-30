use std::{
    error::Error,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    downloader::downloader_sync::downloader_sync,
    task::task::{CrawlTask, creat_task_queue},
};

pub struct CachedInput {
    tasks: Vec<CrawlTask>,
    _server: LocalCacheServer,
}

pub fn prepare_cached_input() -> Result<CachedInput, Box<dyn Error>> {
    let source_tasks = creat_task_queue()?;
    let cache_dir = PathBuf::from("cache");
    fs::create_dir_all(&cache_dir)?;

    let refresh_cache = std::env::var_os("SPIDER_REFRESH_CACHE").is_some();
    let cache_jobs: Vec<_> = source_tasks
        .into_iter()
        .enumerate()
        .map(|(index, task)| {
            let cache_dir = cache_dir.clone();
            thread::spawn(move || {
                let cache_name = format!("{index}.html");
                let cache_path = cache_dir.join(&cache_name);

                if refresh_cache || !cache_path.exists() {
                    if let Err(err) = downloader_sync(&task.url, &cache_path) {
                        eprintln!("cache skip {} ({}): {err}", task.uni, task.url);
                        return None;
                    }
                }

                Some((task.uni, cache_name))
            })
        })
        .collect();

    let mut cached = Vec::new();
    for job in cache_jobs {
        if let Ok(Some(task)) = job.join() {
            cached.push(task);
        }
    }

    let server = LocalCacheServer::start(cache_dir)?;
    let base_url = server.base_url();
    let tasks = cached
        .into_iter()
        .map(|(uni, cache_name)| CrawlTask {
            uni,
            url: format!("{base_url}/{cache_name}"),
        })
        .collect();

    Ok(CachedInput {
        tasks,
        _server: server,
    })
}

impl CachedInput {
    pub fn tasks(&self) -> &[CrawlTask] {
        &self.tasks
    }
}

struct LocalCacheServer {
    address: String,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LocalCacheServer {
    fn start(root: PathBuf) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?.to_string();
        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);

        let handle = thread::spawn(move || {
            while server_running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => handle_cache_request(stream, &root),
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            address,
            running,
            handle: Some(handle),
        })
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for LocalCacheServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = TcpStream::connect(&self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn handle_cache_request(mut stream: TcpStream, root: &Path) {
    let mut request = [0; 2048];
    let Ok(size) = stream.read(&mut request) else {
        return;
    };
    let request = String::from_utf8_lossy(&request[..size]);
    let Some(path) = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
    else {
        write_response(&mut stream, "400 Bad Request", b"bad request");
        return;
    };

    let file_name = path.trim_start_matches('/');
    if !is_cache_file_name(file_name) {
        write_response(&mut stream, "404 Not Found", b"not found");
        return;
    }

    match fs::read(root.join(file_name)) {
        Ok(bytes) => write_response(&mut stream, "200 OK", &bytes),
        Err(_) => write_response(&mut stream, "404 Not Found", b"not found"),
    }
}

fn is_cache_file_name(file_name: &str) -> bool {
    file_name
        .strip_suffix(".html")
        .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|byte| byte.is_ascii_digit()))
}

fn write_response(stream: &mut TcpStream, status: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}
