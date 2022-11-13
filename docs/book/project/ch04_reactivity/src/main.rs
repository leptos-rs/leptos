use leptos::*;

fn main() {
    run_scope(|cx| {
        let (count, set_count) = create_signal(cx, 0);
        let double_count = move || count() * 2;
        let fibonacci = create_memo(cx, |prev| {
            let prev = prev.unwrap_or(1);
            prev * count()
        });

        create_effect(cx, |_| {
            println!(
                "count =\t\t{}\ndouble_count = \t{}\nfibonacci = \t\t{}",
                count(),
                double_count(),
                fibonacci()
            );
        });

        set_count(1);
        set_count(2);
        set_count(3);
    });
}
