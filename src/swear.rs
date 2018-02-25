use std::cell::RefCell;
use std::rc::Rc;

use boxfnonce::BoxFnOnce;

use schedule::Scheduler;

enum SwearState<'a, P> {
    Empty,
    PendingCompletion(BoxFnOnce<'a, (P,)>),
    PendingCallback(P),
    Completed,
}

struct SwearImpl<'a, P, S>
where
    S: Scheduler<'a>,
{
    state: RefCell<SwearState<'a, P>>,
    scheduler: S,
}

impl<'a, P: 'a, S: 'a + Scheduler<'a>> SwearImpl<'a, P, S> {
    fn complete(&self, i: P) {
        // TODO: There may be better options for this... replace_with? (or some sort of map?)
        let s = self.state.replace(SwearState::Empty);
        let new = match s {
            SwearState::Empty => SwearState::PendingCallback(i),
            SwearState::PendingCompletion(cb) => {
                self.scheduler.schedule(move || cb.call(i));
                SwearState::Completed
            }
            SwearState::PendingCallback(_) => panic!("Double completion without callback."),
            SwearState::Completed => panic!("Double completion."),
        };
        self.state.replace(new);
    }

    fn on_complete<F: 'a>(&self, f: F)
    where
        F: FnOnce(P) -> (),
    {
        let s = self.state.replace(SwearState::Empty);
        let new = match s {
            SwearState::Empty => SwearState::PendingCompletion(BoxFnOnce::<'a, (P,)>::from(f)),
            SwearState::PendingCompletion(_) => panic!("Double callback without completion."),
            SwearState::PendingCallback(i) => {
                self.scheduler.schedule(move || f(i));
                SwearState::Completed
            }
            SwearState::Completed => panic!("Double callback."),
        };
        self.state.replace(new);
    }
}

pub struct Completer<'a, P, S>
where
    S: Scheduler<'a>,
{
    s: Rc<SwearImpl<'a, P, S>>,
}

impl<'a, P: 'a, S: 'a + Scheduler<'a>> Completer<'a, P, S> {
    pub fn complete(self, i: P) {
        self.s.complete(i);
    }
}

pub struct Swear<'a, P, S>
where
    S: Scheduler<'a>,
{
    s: Rc<SwearImpl<'a, P, S>>,
}

impl<'a, P: 'a, S: 'a + Scheduler<'a>> Swear<'a, P, S> {
    pub fn then<F: 'a, R: 'a>(self, f: F) -> Swear<'a, R, S>
    where
        F: FnOnce(P) -> R,
    {
        let (rc, rs) = make_swear(self.s.scheduler.clone());
        let newf = move |p: P| rc.complete(f(p));
        self.s.on_complete(newf);
        return rs;
    }

    pub fn and_then<F: 'a, R: 'a>(self, f: F) -> Swear<'a, R, S>
    where
        F: FnOnce(P) -> Swear<'a, R, S>,
    {
        let (rc, rs) = make_swear(self.s.scheduler.clone());
        let newf = move |p: P| {
            f(p).then(move |r| rc.complete(r));
        };
        self.s.on_complete(newf);
        return rs;
    }
}

// first part can be "complete"d, second part is thenable.
pub fn make_swear<'a, P, S: Scheduler<'a>>(s: S) -> (Completer<'a, P, S>, Swear<'a, P, S>) {
    let sw = Rc::new(SwearImpl {
        state: RefCell::new(SwearState::Empty),
        scheduler: s,
    });
    return (Completer { s: sw.clone() }, Swear { s: sw });
}

#[cfg(test)]
mod tests {
    use runqueue::make_runqueue;
    use make_swear;
    use schedule::Scheduler;

    #[test]
    fn try_int_swear() {
        use std::cell::RefCell;
        let i = RefCell::new(0);
        let (q, reactor) = make_runqueue();
        let (sr, se) = make_swear(q.clone());
        let (sr2, se2) = make_swear(q.clone());
        q.schedule(|| *i.borrow_mut() += 1);
        sr2.complete(100);
        se.then(|x| {
            *i.borrow_mut() += x;
            1000
        }).then(|x| {
                *i.borrow_mut() += x;
                "cat"
            })
            .then(|s| {
                if s == "cat" {
                    *i.borrow_mut() += 10000
                }
            });
        se2.then(|x| *i.borrow_mut() += x);
        sr.complete(10);
        reactor.run();
        assert_eq!(11111, *i.borrow());
    }

    #[test]
    fn try_int_swear_with_continuation() {
        use std::cell::RefCell;
        let i = RefCell::new(0);
        let (q, reactor) = make_runqueue();
        let (sr, se) = make_swear(q.clone());
        let (sr2, se2) = make_swear(q.clone());
        q.schedule(|| *i.borrow_mut() += 1);
        sr2.complete(50);
        se.and_then(|x| {
            *i.borrow_mut() += x;
            se2
        }).then(|x| {
                *i.borrow_mut() += x;
                "cat"
            })
            .then(|s| {
                if s == "cat" {
                    *i.borrow_mut() += 10000
                }
            });
        sr.complete(10);
        reactor.run();
        assert_eq!(10061, *i.borrow());
    }
}
