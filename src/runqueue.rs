use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

use boxfnonce::BoxFnOnce;

use schedule::Scheduler;

struct Sender<T> {
    buffer: Rc<RefCell<VecDeque<T>>>
}

impl<T> Sender<T> {
    fn send(&self, t : T) {
        self.buffer.borrow_mut().push_back(t)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        return Sender{
            buffer: Rc::clone(&self.buffer),
        };
    }
}

struct Reciever<T> {
    buffer: Rc<RefCell<VecDeque<T>>>
}

impl<T> Reciever<T> {
    fn recieve(&self) -> Option<T> {
        self.buffer.borrow_mut().pop_front()
    }
}

fn make_channel<T>() -> (Sender<T>, Reciever<T>) {
    let b = Rc::new(RefCell::new(VecDeque::new()));
    (Sender{buffer: Rc::clone(&b)}, Reciever{buffer: b})
}

#[derive(Clone)]
pub struct Queuer<'a> {
    sender: Sender<BoxFnOnce<'a, ()>>
}

impl<'a> Scheduler<'a> for Queuer<'a> {
    fn schedule<F: 'a>(&self, f: F) 
      where F: FnOnce() {
        self.sender.send(BoxFnOnce::<'a, ()>::from(f));
    }
}

pub struct Reactor<'a> {
    receiver: Reciever<BoxFnOnce<'a, ()>>
}

impl<'a> Reactor<'a> {
    pub fn run(&self) {
        while let Some(f) = self.receiver.recieve() {
            f.call();
        }
    }
}

pub fn make_runqueue<'a>() -> (Queuer<'a>, Reactor<'a>) {
    let (sdr, rcv) = make_channel::<BoxFnOnce<()>>();
    return (
        Queuer {
            sender: sdr,
        },
        Reactor {
            receiver: rcv,
        }
    )
}

#[cfg(test)]
mod tests {
    use runqueue::make_channel;
    use runqueue::make_runqueue;
    use schedule::Scheduler;

    #[test]
    fn basic_send_recv() {
        let (sdr, rcv) = make_channel();
        sdr.send(12);
        assert_eq!(12, rcv.recieve().unwrap());
    }

    #[test]
    fn send_recv_with_closures() {
        let mut i = 12;
        {
            let (sdr, rcv) = make_channel();
            sdr.send(|| i += 1);
            let mut f = rcv.recieve().unwrap();
            f();
            f();
        }
        assert_eq!(14, i);
    }

    #[test]
    fn work_queue() {
        use std::cell::RefCell;
        let counter = RefCell::new(0);
        {
            use boxfnonce::BoxFnOnce;
            let (sdr, rcv) = make_channel::<BoxFnOnce<()>>();

            let round2 = || {
                *counter.borrow_mut() += 1;
            };

            let sdrc = sdr.clone();
            let c = &counter;
            let round1 = move || {
                *c.borrow_mut() += 1;
                sdrc.send(BoxFnOnce::from(round2));
            };
            sdr.send(BoxFnOnce::from(round1));

            loop {
                match rcv.recieve() {
                    Some(f) => f.call(),
                    None => break,
                }
            }
        }
        assert_eq!(2, *counter.borrow());
    }

    #[test]
    fn try_reacting() {
        use std::cell::RefCell;
        let i = RefCell::new(0);
        let (q, reactor) = make_runqueue();
        q.schedule(|| *i.borrow_mut() += 1);
        q.schedule(|| *i.borrow_mut() += 1);
        q.schedule(|| *i.borrow_mut() += 1);
        {
            // Hrm... Probably a better way to do this...
            //
            // I want to copy q and move that into the first closure, but I want the second
            // closure to capture i by reference. Explicitly defining variables to be moved into
            // the closures.
            let ii = &i;
            let qq = q.clone();
            q.schedule(move || qq.schedule(move || *ii.borrow_mut() += 1));
        }

        reactor.run();
        assert_eq!(4, *i.borrow());
    }
}
