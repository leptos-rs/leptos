#![feature(test)]

extern crate test;

use leptos::prelude::*;
use test::Bencher;

/// Benchmark the view! macro with different numbers of elements
#[bench]
fn bench_view_macro_single_element(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div>"Hello World"</div>
        };
    });
}

#[bench]
fn bench_view_macro_three_elements(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div>"First"</div>
            <div>"Second"</div>
            <div>"Third"</div>
        };
    });
}

#[bench]
fn bench_view_macro_five_elements(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div>"First"</div>
            <div>"Second"</div>
            <div>"Third"</div>
            <div>"Fourth"</div>
            <div>"Fifth"</div>
        };
    });
}

#[bench]
fn bench_view_macro_ten_elements(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div>"First"</div>
            <div>"Second"</div>
            <div>"Third"</div>
            <div>"Fourth"</div>
            <div>"Fifth"</div>
            <div>"Sixth"</div>
            <div>"Seventh"</div>
            <div>"Eighth"</div>
            <div>"Ninth"</div>
            <div>"Tenth"</div>
        };
    });
}

#[bench]
fn bench_view_macro_with_attributes(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div class="container" id="main" data-test="benchmark">
                <span class="title">"Benchmark Title"</span>
                <p class="description">"This is a benchmark description"</p>
                <button class="btn" disabled=false>"Click me"</button>
            </div>
        };
    });
}

#[bench]
fn bench_view_macro_with_format_macro(b: &mut Bencher) {
    b.iter(|| {
        let href = format!("https://example.com/{}", "test");
        let _view = view! {
            <a href=href class="link">"Formatted Link"</a>
        };
    });
}

#[bench]
fn bench_view_macro_with_self_closing_elements(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <link rel="stylesheet" href="style.css" />
            <meta name="viewport" content="width=device-width" />
            <img src="image.jpg" alt="Test image" />
        };
    });
}

#[bench]
fn bench_view_macro_with_crossorigin_workaround(b: &mut Bencher) {
    b.iter(|| {
        let crossorigin_none: Option<String> = None;
        let _view = view! {
            <link rel="preload" href="script.js" crossorigin=crossorigin_none />
        };
    });
}

/// Benchmark tuple generation for different element counts
#[bench]
fn bench_tuple_generation_three_elements(b: &mut Bencher) {
    b.iter(|| {
        let elements = vec![
            quote! { <div>"First"</div> },
            quote! { <div>"Second"</div> },
            quote! { <div>"Third"</div> },
        ];
        let _tuple = quote! {
            (#(#elements),*)
        };
    });
}

#[bench]
fn bench_tuple_generation_five_elements(b: &mut Bencher) {
    b.iter(|| {
        let elements = vec![
            quote! { <div>"First"</div> },
            quote! { <div>"Second"</div> },
            quote! { <div>"Third"</div> },
            quote! { <div>"Fourth"</div> },
            quote! { <div>"Fifth"</div> },
        ];
        let chunks = elements.chunks(3).map(|chunk| {
            quote! {
                (#(#chunk),*)
            }
        });
        let _tuple = quote! {
            (#(#chunks),*)
        };
    });
}

#[bench]
fn bench_tuple_generation_ten_elements(b: &mut Bencher) {
    b.iter(|| {
        let elements = vec![
            quote! { <div>"First"</div> },
            quote! { <div>"Second"</div> },
            quote! { <div>"Third"</div> },
            quote! { <div>"Fourth"</div> },
            quote! { <div>"Fifth"</div> },
            quote! { <div>"Sixth"</div> },
            quote! { <div>"Seventh"</div> },
            quote! { <div>"Eighth"</div> },
            quote! { <div>"Ninth"</div> },
            quote! { <div>"Tenth"</div> },
        ];
        let chunks = elements.chunks(3).map(|chunk| {
            quote! {
                (#(#chunk),*)
            }
        });
        let _tuple = quote! {
            (#(#chunks),*)
        };
    });
}

/// Benchmark macro expansion time
#[bench]
fn bench_macro_expansion_simple(b: &mut Bencher) {
    b.iter(|| {
        // This would need to be implemented with actual macro expansion
        // For now, we'll simulate the token generation
        let tokens = quote! {
            view! {
                <div>"Hello"</div>
            }
        };
        let _expanded = tokens;
    });
}

/// Benchmark hydration-specific scenarios
#[bench]
fn bench_hydration_module_structure(b: &mut Bencher) {
    b.iter(|| {
        let root = "http://localhost:3000";
        let pkg_path = "pkg";
        let js_file_name = "app";
        let wasm_file_name = "app";
        let script = "import";
        let islands_router = "";
        let nonce = None::<String>;
        let crossorigin_none: Option<String> = None;

        let _view = view! {
            <link rel="modulepreload" href=format!("{root}/{pkg_path}/{js_file_name}.js") crossorigin=nonce.clone()/>
            <link rel="preload" href=format!("{root}/{pkg_path}/{wasm_file_name}.wasm") crossorigin=crossorigin_none />
            <script type="module" nonce=nonce>
                {format!("{script}({root:?}, {pkg_path:?}, {js_file_name:?}, {wasm_file_name:?});{islands_router}")}
            </script>
        };
    });
}

/// Benchmark memory usage patterns
#[bench]
fn bench_memory_usage_large_view(b: &mut Bencher) {
    b.iter(|| {
        // Create a large view with many elements
        let mut elements = Vec::new();
        for i in 0..100 {
            elements.push(format!("Element {}", i));
        }
        
        let _view = view! {
            <div class="container">
                {elements.into_iter().map(|text| {
                    view! {
                        <div class="item">{text}</div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        };
    });
}

/// Benchmark attribute processing
#[bench]
fn bench_attribute_processing(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div 
                class="container"
                id="main"
                data-test="benchmark"
                data-value="123"
                data-flag=true
                data-optional=None::<String>
                style="color: red; font-size: 16px;"
                onclick="handleClick()"
                onmouseover="handleHover()"
                aria-label="Benchmark element"
                role="button"
                tabindex="0"
            >
                "Content with many attributes"
            </div>
        };
    });
}

/// Benchmark conditional rendering
#[bench]
fn bench_conditional_rendering(b: &mut Bencher) {
    b.iter(|| {
        let show_extra = true;
        let count = 5;
        
        let _view = view! {
            <div>
                <h1>"Conditional Benchmark"</h1>
                {if show_extra {
                    view! {
                        <div class="extra">
                            <p>"This is extra content"</p>
                            <ul>
                                {(0..count).map(|i| {
                                    view! {
                                        <li>"Item {i}"</li>
                                    }
                                }).collect::<Vec<_>>()}
                            </ul>
                        </div>
                    }
                } else {
                    view! { <div>"No extra content"</div> }
                }}
            </div>
        };
    });
}

/// Benchmark list rendering
#[bench]
fn bench_list_rendering(b: &mut Bencher) {
    b.iter(|| {
        let items = (0..50).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
        
        let _view = view! {
            <ul class="list">
                {items.into_iter().map(|item| {
                    view! {
                        <li class="list-item">{item}</li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        };
    });
}

/// Benchmark nested component structure
#[bench]
fn bench_nested_structure(b: &mut Bencher) {
    b.iter(|| {
        let _view = view! {
            <div class="page">
                <header class="header">
                    <nav class="nav">
                        <ul class="nav-list">
                            <li class="nav-item"><a href="/">"Home"</a></li>
                            <li class="nav-item"><a href="/about">"About"</a></li>
                            <li class="nav-item"><a href="/contact">"Contact"</a></li>
                        </ul>
                    </nav>
                </header>
                <main class="main">
                    <article class="article">
                        <h1 class="title">"Article Title"</h1>
                        <p class="content">"Article content goes here..."</p>
                        <footer class="article-footer">
                            <span class="author">"By Author"</span>
                            <time class="date">"2024-01-01"</time>
                        </footer>
                    </article>
                </main>
                <footer class="footer">
                    <p class="copyright">"© 2024 Company"</p>
                </footer>
            </div>
        };
    });
}
