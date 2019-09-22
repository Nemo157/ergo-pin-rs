use core::pin::Pin;
use ergo_pin::ergo_pin;

struct LessThan5(usize);

impl LessThan5 {
    fn check(self: Pin<&mut Self>) -> bool {
        self.0 < 5
    }
}

#[test]
fn multi_evaluated_expressions() {
    #[ergo_pin]
    fn count_to_5() -> usize {
        let mut count = 0;
        while pin!(LessThan5(count)).check() {
            count += 1;
        }
        count
    }

    #[ergo_pin]
    fn count_to_6() -> usize {
        let mut count = 0;
        while let true = pin!(LessThan5(count)).check() {
            count += 1;
        }
        count + 1
    }

    assert_eq!(count_to_5(), 5);
    assert_eq!(count_to_6(), 6);
}
