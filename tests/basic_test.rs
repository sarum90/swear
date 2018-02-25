extern crate swear;

use swear::runqueue::make_runqueue;
use swear::runqueue::Queuer;
use swear::runqueue::Reactor;
use swear::schedule::Scheduler;
use swear::Swear;
use swear::make_swear;

#[derive(Clone)]
struct Runner<'a> {
    q: Queuer<'a>,
}

// Hopefully it's not too much of a stretch of the imagination to imagine an implementation of this
// that actually runs an epoll loop on raw connections. If that were to exist, I would certainly
// write it in a different crate.
impl<'a> Runner<'a> {
    fn make() -> (Runner<'a>, Reactor<'a>) {
        let (q, r) = make_runqueue();
        return (Runner { q: q }, r);
    }

    // Fakes like an RPC with value va.
    fn make_value_swear<V: 'a>(&self, va: V) -> Swear<'a, V, Queuer<'a>> {
        let (c, s) = make_swear(self.q.clone());
        self.q.schedule(move || {
            c.complete(va);
        });
        return s;
    }
}

#[test]
fn simple_usage() {
    let (r, rea) = Runner::make();
    r.make_value_swear(2).then(|x| assert_eq!(x, 2));
    rea.run();
}

#[test]
fn chained_rpcs() {
    let (r, rea) = Runner::make();
    let rr = r.clone();
    r.make_value_swear(2)
        .and_then(move |x| rr.make_value_swear(x + 2))
        .then(|x| assert_eq!(x, 4));
    rea.run();
}
