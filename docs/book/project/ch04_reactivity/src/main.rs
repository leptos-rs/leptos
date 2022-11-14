use leptos::*;

fn main() {
    run_scope(|cx| {
        // signal
        let (count, set_count) = create_signal(cx, 1);

        // derived signal
        let double_count = move || count() * 2;

        // memo
        let memoized_square = create_memo(cx, move |_| count() * count());

        // effect
        create_effect(cx, move |_| {
            println!(
                "count =\t\t{} \ndouble_count = \t{}, \nsquare = \t{}",
                count(),
                double_count(),
                memoized_square()
            );
        });

        set_count(1);
        set_count(2);
        set_count(3);
    });
}
