lazy_static::lazy_static! {
    pub static ref CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
}

use hhmmss::Hhmmss;

#[derive(Debug)]
pub struct Message {
    pub user: String,
    pub body: String,
    pub timestamp: f64,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let seconds = std::time::Duration::from_secs(self.timestamp as u64);
        write!(f, "[{}][{}]: {}", seconds.hhmmss(), self.user, self.body)
    }
}

pub trait Vod: std::fmt::Display {
    fn comments(&self) -> Box<dyn ChatIterator>;
}

pub trait ChatIterator: Send + Iterator<Item = Vec<Message>> {
    /// Will walk the ChatIterator and save the output into a buffer.
    /// When display_sig recieves a signal, the buffer will be flushed into stdout
    fn display_worker(
        &mut self,
        display_sig: std::sync::mpsc::Receiver<()>,
        filter: &regex::Regex,
    ) {
        let mut display_now = false;
        let mut buf = Vec::new();
        for message in self
            .flatten()
            .filter(|message| filter.is_match(&message.body))
        {
            buf.push(message);
            if display_now {
                buf.iter().for_each(|m| println!("{}", m));
                buf.clear();
            } else if display_sig.try_recv().is_ok() {
                display_now = true;
            }
        }
        if !display_now {
            display_sig.recv().unwrap();
            buf.iter().for_each(|m| println!("{}", m));
        }
    }
}

pub fn print_iter<V>(vods: &[V], filter: regex::Regex)
where
    V: Vod,
{
    let filter = &filter;
    std::thread::scope(|t| {
        let mut future_manager = Vec::with_capacity(vods.len());

        for vod in vods {
            let mut comments = vod.comments();
            let (tx, rx) = std::sync::mpsc::channel();
            let fut = t.spawn(move || comments.display_worker(rx, filter));
            future_manager.push((vod, fut, tx));
        }

        for (vod, fut, tx) in future_manager {
            println!("{}", vod);
            tx.send(()).unwrap();
            fut.join().unwrap();
            println!();
        }
    });
}
