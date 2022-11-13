use leptos::*;

fn main() {
    run_scope(|cx| {
        let (count, set_count) = create_signal(cx, 1);
        let double_count = move || count() * 2;

        create_effect(cx, move |_| {
            println!(
                "count =\t\t{}\ndouble_count = \t{}",
                count(),
                double_count(),
            );
        });

        set_count(1);
        set_count(2);
        set_count(3);
    });
}
