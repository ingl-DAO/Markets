use solana_program::entrypoint;

pub mod error;
pub mod instruction;
pub mod processes;
pub mod processor;
pub mod state;
pub mod utils;

use processor::process_instruction;

entrypoint!(process_instruction);

#[cfg(test)]
mod tests {
    struct Base {
        left: u64,
    }

    fn scope1(tmp: Base) {
        println!("scope1: {}", tmp.left);
    }

    fn scope2(tmp: &Base) {
        println!("scope 2: {}", tmp.left);
    }

    fn primitive1(tmp: [u64; 1]) {
        println!("primitive 1: {:?}", tmp);
    }

    fn primitive2(tmp: &[u64; 1]) {
        println!("primitive 2: {:?}", tmp);
    }

    #[test]
    fn scope3() {
        let tmp = Base { left: 1 };
        scope2(&tmp);
        scope1(tmp);
        // scope2(&tmp); // error: use of moved value: `tmp`

        let a = [10];
        primitive2(&a);
        primitive1(a);
        primitive2(&a);
        primitive1(a);

        // primitive types copy by default.
    }

    fn scope4(tmp: &mut Base) {
        tmp.left += 1;
        println!("scope 4, tmp.left: {}", tmp.left);
    }

    fn scope5(tmp: &mut u64) {
        *tmp += *tmp;
        println!("scope 5, a: {}", tmp);
    }

    #[test]
    fn scope6() {
        let mut tmp = Base { left: 1 };
        let a = &mut 10;
        scope4(&mut tmp);
        scope5(a);
        println!("scope 6, tmp.left: {}", tmp.left);
        println!("scope 6, a: {}", a);
    }
}
