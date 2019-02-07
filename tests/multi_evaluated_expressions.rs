#![feature(generators, generator_trait)]

use core::ops::{Generator, GeneratorState};
use ergo_pin::ergo_pin;

#[test]
fn multi_evaluated_expressions() {
    fn less_than_5(i: usize) -> impl Generator<Yield = bool, Return = ()> {
        static move || yield i < 5
    }

    #[ergo_pin]
    fn count_to_5() -> usize {
        let mut count = 0;
        while pin!(less_than_5(count)).resume() == GeneratorState::Yielded(true) {
            count += 1;
        }
        return count;
    }

    #[ergo_pin]
    fn count_to_6() -> usize {
        let mut count = 0;
        while let GeneratorState::Yielded(true) = pin!(less_than_5(count)).resume() {
            count += 1;
        }
        return count + 1;
    }

    assert_eq!(count_to_5(), 5);
    assert_eq!(count_to_6(), 6);
}
