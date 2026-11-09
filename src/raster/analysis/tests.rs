use super::*;

fn solid(w: u32, h: u32, color: [u8; 4]) -> RgbaImage {
    let mut image = RgbaImage::new(w, h);
    image.fill_rect(0, 0, w, h, color);
    image
}

fn tiled(
    tiles: &[RgbaImage],
    cols: u32,
    rows: u32,
    pattern: &[usize],
    offset: [u32; 2],
    spacing: u32,
) -> (RgbaImage, Grid) {
    let tile = tiles[0].width;
    let grid = Grid {
        tile,
        offset,
        spacing,
    };
    let (last_x, last_y) = grid.origin(cols - 1, rows - 1);
    let mut image = solid(last_x + tile, last_y + tile, [251, 3, 201, 255]);
    for row in 0..rows {
        for col in 0..cols {
            let source = &tiles[pattern[(row * cols + col) as usize]];
            let (x, y) = grid.origin(col, row);
            for dy in 0..tile {
                for dx in 0..tile {
                    image.set_pixel(x + dx, y + dy, source.pixel(dx, dy));
                }
            }
        }
    }
    (image, grid)
}

#[test]
fn alpha_key_and_hex_are_exact() {
    let mut image = RgbaImage {
        width: 4,
        height: 1,
        pixels: vec![9, 8, 7, 0, 1, 2, 3, 1, 255, 0, 255, 12, 255, 1, 255, 255],
    };
    image.normalize_alpha();
    assert_eq!(image.pixel(0, 0), [0; 4]);
    assert_eq!(image.pixel(1, 0), [1, 2, 3, 1]);
    let normalized = image.clone();
    image.normalize_alpha();
    assert_eq!(image, normalized);
    image.apply_key_color([255, 0, 255]);
    assert_eq!(image.pixel(2, 0), [0; 4]);
    assert_eq!(image.pixel(3, 0), [255, 1, 255, 255]);
    assert_eq!(parse_hex_color("ff00ff"), Ok([255, 0, 255]));
    assert_eq!(parse_hex_color("#FF00FF"), Ok([255, 0, 255]));
    for bad in ["ff00f", "gg0000", "#ff00ff00", "é0000", "##00000"] {
        assert!(parse_hex_color(bad).is_err(), "{bad}");
    }
}

#[test]
fn tile_bounds_and_alpha_only_emptiness() {
    let grid = Grid {
        tile: 16,
        offset: [0, 0],
        spacing: 0,
    };
    let mut image = solid(256, 64, [9, 8, 7, 0]);
    assert!(image.tile(&grid, 15, 3).is_some());
    assert!(image.tile(&grid, 16, 3).is_none());
    assert!(image.tile(&grid, 0, 4).is_none());
    assert!(image.tile_is_empty(&grid, 15, 3));
    image.set_pixel(255, 63, [1, 2, 3, 1]);
    assert!(!image.tile_is_empty(&grid, 15, 3));
}

#[test]
fn dedupe_is_normalized_ordered_and_spacing_aware() {
    let tiles: Vec<_> = (1..=3).map(|c| solid(2, 2, [c, 0, 0, 255])).collect();
    for spacing in [0, 1, 5] {
        let (image, grid) = tiled(&tiles, 4, 2, &[0, 1, 0, 2, 1, 0, 2, 2], [3, 5], spacing);
        let result = dedupe(&image, &grid);
        assert_eq!(result.unique, tiles);
        assert_eq!(result.map, [0, 1, 0, 2, 1, 0, 2, 2]);
        assert_eq!(result.counts, [3, 2, 3]);
        assert_eq!(rebuild_matches(&image, &grid, &result), Ok(()));
        let mut wrong = result.clone();
        wrong.map[2] = 1;
        assert_eq!(rebuild_matches(&image, &grid, &wrong), Err(vec![(2, 0)]));
        let mut wrong = result.clone();
        wrong.unique[0].set_pixel(0, 0, [9; 4]);
        assert_eq!(
            rebuild_matches(&image, &grid, &wrong),
            Err(vec![(0, 0), (2, 0), (1, 1)])
        );
        let mut wrong = result.clone();
        wrong.map[0] = usize::MAX;
        assert_eq!(rebuild_matches(&image, &grid, &wrong), Err(vec![(0, 0)]));
    }
    let (image, grid) = tiled(
        &[solid(2, 2, [9, 9, 9, 0]), solid(2, 2, [0; 4])],
        2,
        1,
        &[0, 1],
        [0, 0],
        1,
    );
    let result = dedupe(&image, &grid);
    assert_eq!(result.unique, vec![solid(2, 2, [0; 4])]);
    assert_eq!(result.map, [0, 0]);
    assert_eq!(result.counts, [2]);
}

#[test]
fn sheets_clamp_columns_and_pad_transparently() {
    let tiles: Vec<_> = (1..=5).map(|c| solid(2, 2, [c, 0, 0, 255])).collect();
    let wide = build_sheet(&tiles, 2, 16);
    assert_eq!((wide.width, wide.height), (10, 2));
    assert_eq!(wide.pixel(8, 0), [5, 0, 0, 255]);
    let sheet = build_sheet(&tiles, 2, 2);
    assert_eq!((sheet.width, sheet.height), (4, 6));
    assert_eq!(sheet.pixel(0, 4), [5, 0, 0, 255]);
    assert_eq!(sheet.pixel(2, 4), [0; 4]);
    assert_eq!(build_sheet(&[], 16, 16), solid(16, 16, [0; 4]));
    assert_eq!(build_sheet(&tiles, 2, 0).height, 10);
}

fn patterned_tiles() -> Vec<RgbaImage> {
    (0..4)
        .map(|id| {
            let mut tile = RgbaImage::new(16, 16);
            for y in 0..16 {
                for x in 0..16 {
                    tile.set_pixel(
                        x,
                        y,
                        [
                            (x * 13 + id * 37) as u8,
                            (y * 11 + id * 19) as u8,
                            (x * y + id * 47) as u8,
                            255,
                        ],
                    );
                }
            }
            tile
        })
        .collect()
}

#[test]
fn guess_finds_independent_offsets_and_larger_repeating_tile() {
    let pattern: Vec<_> = (0..72).map(|i| ((i * 7 + i / 12) % 4) as usize).collect();
    let (image, _) = tiled(&patterned_tiles(), 12, 6, &pattern, [3, 5], 0);
    let guess = guess_grid(&image, &[8, 16], true);
    assert_eq!(guess.pick, Some((16, (3, 5))));
    let unshifted = guess_grid(&image, &[8, 16], false);
    assert!(unshifted.scores.iter().all(|s| s.offset == (0, 0)));
    let (image, _) = tiled(&patterned_tiles(), 12, 6, &pattern, [0, 0], 0);
    let guess = guess_grid(&image, &[8, 16], false);
    assert!(guess.scores.iter().all(|s| s.ratio <= REPEAT_RATIO));
    assert_eq!(guess.pick, Some((16, (0, 0))));
}

#[test]
fn guess_threshold_fallback_empty_and_ties() {
    let (image, _) = tiled(&patterned_tiles(), 4, 4, &[0, 1, 2, 3].repeat(4), [0, 0], 0);
    let guess = guess_grid(&image, &[16], false);
    assert_eq!(guess.scores[0].ratio, 0.25);
    assert_eq!(guess.pick, Some((16, (0, 0))));
    let mut noise = RgbaImage::new(64, 64);
    for y in 0..64 {
        for x in 0..64 {
            noise.set_pixel(x, y, [x as u8, y as u8, 0, 255]);
        }
    }
    let guess = guess_grid(&noise, &[16, 8], false);
    assert!(guess.scores.iter().all(|s| s.ratio > REPEAT_RATIO));
    assert_eq!(guess.pick, Some((8, (0, 0)))); // equal unique*area: smaller tile
    // Partial edge tiles are discarded: 12px has less unique pixel area here.
    assert_eq!(guess_grid(&noise, &[8, 12], false).pick, Some((12, (0, 0))));
    let image = solid(64, 64, [1; 4]);
    assert_eq!(guess_grid(&image, &[8], true).pick, Some((8, (0, 0))));
    assert_eq!(
        guess_grid(&solid(7, 7, [0; 4]), &DEFAULT_GUESS_CANDIDATES, true),
        GridGuess {
            pick: None,
            scores: vec![]
        }
    );
    assert_eq!(guess_grid(&image, &[0, u32::MAX], true).pick, None);
    let (image, _) = tiled(
        &[solid(8, 8, [9, 9, 9, 0]), solid(8, 8, [0; 4])],
        4,
        4,
        &[0, 1].repeat(8),
        [0, 0],
        0,
    );
    assert_eq!(guess_grid(&image, &[8], false).scores[0].unique, 1);
}

#[test]
fn objects_use_eight_neighbors_background_and_sorted_bounds() {
    let mut image = RgbaImage::new(30, 30);
    image.fill_rect(20, 0, 10, 4, [255; 4]);
    image.fill_rect(0, 10, 4, 5, [255; 4]);
    image.fill_rect(10, 20, 3, 3, [255; 4]);
    let expected = vec![
        ObjectBox {
            x: 20,
            y: 0,
            w: 10,
            h: 4,
        },
        ObjectBox {
            x: 0,
            y: 10,
            w: 4,
            h: 5,
        },
    ];
    assert_eq!(find_objects(&image, Background::Transparent, 4), expected);
    assert_eq!(find_objects(&image, Background::Auto, 4), expected);
    let mut diagonal = RgbaImage::new(2, 2);
    diagonal.set_pixel(0, 0, [1; 4]);
    diagonal.set_pixel(1, 1, [1; 4]);
    assert_eq!(
        find_objects(&diagonal, Background::Transparent, 1),
        vec![ObjectBox {
            x: 0,
            y: 0,
            w: 2,
            h: 2
        }]
    );
    let mut panels = solid(200, 200, [255, 190, 220, 255]);
    panels.fill_rect(5, 10, 60, 80, [255, 0, 255, 255]);
    panels.fill_rect(100, 90, 100, 110, [255, 0, 255, 255]);
    panels.set_pixel(20, 20, [0, 0, 255, 255]);
    assert_eq!(
        find_objects(&panels, Background::Auto, 32),
        vec![
            ObjectBox {
                x: 5,
                y: 10,
                w: 60,
                h: 80
            },
            ObjectBox {
                x: 100,
                y: 90,
                w: 100,
                h: 110
            }
        ]
    );
    assert_eq!(
        find_objects(&solid(200, 200, [255; 4]), Background::Transparent, 4),
        vec![ObjectBox {
            x: 0,
            y: 0,
            w: 200,
            h: 200
        }]
    );
    let mut alpha = solid(2, 1, [1, 2, 3, 0]);
    alpha.set_pixel(1, 0, [1, 2, 3, 1]);
    assert_eq!(
        find_objects(&alpha, Background::Color([1, 2, 3, 0]), 1),
        vec![ObjectBox {
            x: 1,
            y: 0,
            w: 1,
            h: 1
        }]
    );
    assert!(find_objects(&RgbaImage::new(0, 0), Background::Auto, 4).is_empty());
}

#[test]
fn snap_clips_to_cells_and_skips_gaps_and_leftovers() {
    let grid = Grid {
        tile: 16,
        offset: [0, 0],
        spacing: 0,
    };
    assert_eq!(
        ObjectBox {
            x: 17,
            y: 0,
            w: 20,
            h: 10
        }
        .snap(&grid, 64, 32),
        Some(CellRect {
            col: 1,
            row: 0,
            cols: 2,
            rows: 1
        })
    );
    let grid = Grid {
        offset: [2, 3],
        spacing: 1,
        ..grid
    };
    assert_eq!(
        ObjectBox {
            x: 18,
            y: 3,
            w: 20,
            h: 10
        }
        .snap(&grid, 70, 40),
        Some(CellRect {
            col: 1,
            row: 0,
            cols: 2,
            rows: 1
        })
    );
    assert_eq!(
        ObjectBox {
            x: 0,
            y: 0,
            w: 4,
            h: 5
        }
        .snap(&grid, 70, 40),
        Some(CellRect {
            col: 0,
            row: 0,
            cols: 1,
            rows: 1
        })
    );
    assert_eq!(
        ObjectBox {
            x: 69,
            y: 3,
            w: 1,
            h: 10
        }
        .snap(&grid, 70, 40),
        None
    );
    assert_eq!(
        ObjectBox {
            x: 0,
            y: 0,
            w: 1,
            h: 1
        }
        .snap(&grid, 70, 40),
        None
    );
    assert_eq!(
        ObjectBox {
            x: u32::MAX,
            y: 3,
            w: u32::MAX,
            h: 10
        }
        .snap(&grid, 70, 40),
        None
    );
    assert_eq!(
        ObjectBox {
            x: 2,
            y: 3,
            w: 16,
            h: 16
        }
        .snap(&grid, 70, 40),
        Some(CellRect {
            col: 0,
            row: 0,
            cols: 1,
            rows: 1
        })
    );
}

#[test]
#[ignore = "timing report; explicit local run only"]
fn guess_timing_report() {
    if std::env::var_os("RN_RASTER_TIMING").is_none() {
        println!("rn: skipped timing report (set RN_RASTER_TIMING=1)");
        return;
    }
    let image = solid(256, 144, [1, 2, 3, 255]);
    let start = std::time::Instant::now();
    let result = guess_grid(&image, &DEFAULT_GUESS_CANDIDATES, true);
    println!(
        "rn: synthetic 256x144 guess {:?}, pick={:?}",
        start.elapsed(),
        result.pick
    );
}

#[test]
#[ignore = "local human assets; requires RN_LOCAL_ASSETS=1"]
fn local_assets_report() {
    if std::env::var_os("RN_LOCAL_ASSETS").as_deref() != Some(std::ffi::OsStr::new("1")) {
        println!("rn: skipped local assets (set RN_LOCAL_ASSETS=1)");
        return;
    }
    // The ignored assets live in the primary checkout; a worktree is <root>/.worktrees/<name>.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = match manifest.parent() {
        Some(parent) if parent.ends_with(".worktrees") => parent.parent().unwrap(),
        _ => manifest,
    };
    let mut paths = vec![
        std::path::PathBuf::from("/home/raven/Works/splites/splite-cave-01.png"),
        std::path::PathBuf::from("/home/raven/Works/splites/splite-cave-02.png"),
    ];
    let cave = root.join("assets/splites/cave");
    match std::fs::read_dir(&cave) {
        Ok(entries) => paths.extend(
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "png")),
        ),
        Err(_) => println!("rn: skipped {}", cave.display()),
    }
    paths.sort();
    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else {
            println!("rn: skipped {}", path.display());
            continue;
        };
        let image = RgbaImage::from_png(&bytes).unwrap();
        let grid = Grid {
            tile: 16,
            offset: [0, 0],
            spacing: 0,
        };
        let cells = grid.cells(image.width, image.height);
        if path.starts_with(root.join("assets/splites/cave")) {
            assert!([(256, 64), (256, 144)].contains(&(image.width, image.height)));
            assert_eq!((cells.cols, cells.rows), (16, image.height / 16));
        }
        let unique = dedupe(&image, &grid);
        println!(
            "rn: {} {}x{} cells={}x{} unique={} rebuild={:?}",
            path.display(),
            image.width,
            image.height,
            cells.cols,
            cells.rows,
            unique.unique.len(),
            rebuild_matches(&image, &grid, &unique)
        );
        if path.file_name().is_some_and(|s| s == "splite-cave-02.png") {
            let start = std::time::Instant::now();
            let guess = guess_grid(&image, &DEFAULT_GUESS_CANDIDATES, true);
            println!(
                "rn: map guess {:?} pick={:?} scores={:?}",
                start.elapsed(),
                guess.pick,
                guess.scores
            );
        }
    }
}

#[test]
fn guess_work_counts_only_valid_offsets_and_progress_reaches_the_total() {
    // 5x6: tile=2 allows 2 x 2 offsets; tile=3 has fewer than two columns.
    let image = RgbaImage::new(5, 6);
    assert_eq!(guess_grid_work(&image, &[0, 2, 3, u32::MAX], true), 4);
    assert_eq!(guess_grid_work(&image, &[2, 3], false), 1);
    assert_eq!(guess_grid_work(&RgbaImage::new(6, 7), &[3], true), 2);
    for width in 0..12 {
        for height in 0..12 {
            let image = RgbaImage::new(width, height);
            for search in [false, true] {
                let candidates = [0, 1, 2, 3, 5, 8];
                let mut expected = 0;
                for tile in candidates {
                    for oy in 0..if search { tile } else { 1 } {
                        for ox in 0..if search { tile } else { 1 } {
                            let cells = Grid {
                                tile,
                                offset: [ox, oy],
                                spacing: 0,
                            }
                            .cells(width, height);
                            expected += u64::from(cells.cols >= 2 && cells.rows >= 2);
                        }
                    }
                }
                assert_eq!(guess_grid_work(&image, &candidates, search), expected);
            }
        }
    }
    let ticks = std::cell::RefCell::new(Vec::new());
    let result = guess_grid_with(
        &image,
        &[0, 2, 3],
        true,
        &|done, total| ticks.borrow_mut().push((done, total)),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(*ticks.borrow(), [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]);
    assert_eq!(result, guess_grid(&image, &[0, 2, 3], true));
    assert_eq!(
        result.scores,
        vec![GridScore {
            tile: 2,
            offset: (0, 0),
            unique: 1,
            total: 6,
            ratio: 1.0 / 6.0,
        }]
    );
    assert_eq!(guess_grid_work(&RgbaImage::new(0, 0), &[2], true), 0);
}

#[test]
fn guess_cancellation_discards_partial_scores() {
    let image = RgbaImage::new(12, 12);
    let cancel = AtomicBool::new(true);
    assert!(guess_grid_with(&image, &[2], true, &|_, _| {}, &cancel).is_none());
    cancel.store(false, Ordering::Relaxed);
    let visited = std::cell::Cell::new(0);
    let result = guess_grid_with(
        &image,
        &[2, 3],
        true,
        &|done, _| {
            visited.set(done);
            if done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
        &cancel,
    );
    assert!(result.is_none());
    assert_eq!(visited.get(), 1);
}
