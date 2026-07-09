//! Canned demo data generation (03-features/demo-mode §5.2). Dish pools transcribed verbatim
//! from v1 demoData.ts; menu generation uses the deterministic LCG (§5.1) so results are
//! stable within a calendar day and rotate daily.
use crate::datetime::Clock;
use crate::demo::prng::Lcg;
use crate::domain::{Bill, BillingItem, MenuCategory, MenuItem, VentopayTransaction};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};

const BILLING_DESCRIPTIONS: [&str; 4] = ["Menü I", "Menü II", "Menü III", "Suppe & Salat"];
const BILLING_PRICES: [f64; 10] = [6.80, 5.90, 6.20, 4.20, 5.50, 6.50, 5.80, 6.00, 4.80, 5.20];
const TX_AMOUNTS: [f64; 6] = [0.50, 1.00, 1.20, 1.50, 2.00, 2.50];

/// A dish: (title, subtitle, allergen letters).
type Dish = (&'static str, &'static str, &'static [&'static str]);

struct Pool {
    category: MenuCategory,
    id_prefix: &'static str,
    price: &'static str,
    dishes: [Dish; 10],
}

const MENU1: Pool = Pool {
    category: MenuCategory::Menu1,
    id_prefix: "demo-m1",
    price: "6,00 €",
    dishes: [
        (
            "Wiener Schnitzel",
            "mit Petersilerdäpfel und Preiselbeeren",
            &["A", "C", "G"],
        ),
        (
            "Schweinsbraten",
            "mit Semmelknödel und Sauerkraut",
            &["A", "C", "G"],
        ),
        (
            "Tafelspitz",
            "mit Apfelkren und Schnittlauchsauce",
            &["A", "G", "L"],
        ),
        (
            "Rindsgulasch",
            "mit Nockerl und Essiggurkerl",
            &["A", "C", "G"],
        ),
        ("Backhendl", "mit Erdäpfelsalat", &["A", "C", "G"]),
        (
            "Cordon Bleu",
            "mit Reis und Preiselbeeren",
            &["A", "C", "G"],
        ),
        (
            "Zwiebelrostbraten",
            "mit Bratkartoffeln und Röstzwiebeln",
            &["A", "G", "L"],
        ),
        (
            "Faschierter Braten",
            "mit Erdäpfelpüree und Bratensauce",
            &["A", "C", "G"],
        ),
        ("Kalbsrahmgulasch", "mit Butternockerl", &["A", "C", "G"]),
        (
            "Gebackene Leber",
            "mit Erdäpfelsalat und Preiselbeeren",
            &["A", "C", "G"],
        ),
    ],
};

const MENU2: Pool = Pool {
    category: MenuCategory::Menu2,
    id_prefix: "demo-m2",
    price: "6,00 €",
    dishes: [
        ("Gemüselasagne", "mit Blattsalat", &["A", "C", "G"]),
        (
            "Spinatknödel",
            "mit Parmesan und brauner Butter",
            &["A", "C", "G"],
        ),
        (
            "Käsespätzle",
            "mit Röstzwiebeln und grünem Salat",
            &["A", "C", "G"],
        ),
        (
            "Pasta Primavera",
            "mit Saisongemüse und Basilikum",
            &["A", "C"],
        ),
        ("Kartoffelgratin", "mit buntem Gemüse", &["A", "G"]),
        (
            "Topfenknödel",
            "mit Butterbröseln und Apfelmus",
            &["A", "C", "G"],
        ),
        ("Gemüse-Curry", "mit Basmatireis und Naan-Brot", &["A", "G"]),
        (
            "Flammkuchen",
            "mit Sauerrahm, Zwiebeln und Speck",
            &["A", "G"],
        ),
        (
            "Eierschwammerlgulasch",
            "mit Semmelknödel",
            &["A", "C", "G"],
        ),
        (
            "Palatschinken",
            "mit Topfenfülle und Vanillesauce",
            &["A", "C", "G"],
        ),
    ],
};

const MENU3: Pool = Pool {
    category: MenuCategory::Menu3,
    id_prefix: "demo-m3",
    price: "6,00 €",
    dishes: [
        (
            "Grillhendl",
            "mit Pommes frites und Cole Slaw",
            &["A", "G", "M"],
        ),
        (
            "Fischfilet",
            "mit Dillsauce und Petersilerdäpfel",
            &["A", "C", "D", "G"],
        ),
        (
            "Putengeschnetzeltes",
            "mit Reis und Champignons",
            &["A", "G"],
        ),
        ("Cevapcici", "mit Djuvec-Reis und Ajvar", &["A", "C"]),
        (
            "Hühnercurry",
            "mit Jasminreis und Mango-Chutney",
            &["A", "G"],
        ),
        (
            "Leberkäse",
            "mit Spiegelei und Erdäpfelsalat",
            &["A", "C", "G"],
        ),
        ("Bratwürstel", "mit Senf und Sauerkraut", &["A", "M"]),
        (
            "Puten-Wrap",
            "mit Salat, Tomaten und Joghurt-Dressing",
            &["A", "G"],
        ),
        (
            "Lachs gegrillt",
            "mit Zitronenbutter und Gemüsereis",
            &["D", "G"],
        ),
        ("Spaghetti Bolognese", "mit Parmesan", &["A", "C", "G"]),
    ],
};

const SOUP_SALAD: Pool = Pool {
    category: MenuCategory::SoupAndSalad,
    id_prefix: "demo-ss",
    price: "2,50 €",
    dishes: [
        (
            "Frittatensuppe",
            "Klare Rindsuppe mit Frittaten",
            &["A", "C", "G"],
        ),
        (
            "Kürbiscremesuppe",
            "mit Kürbiskernöl und Croutons",
            &["A", "G"],
        ),
        ("Gemischter Salat", "mit Hausdressing", &["M"]),
        (
            "Grießnockerlsuppe",
            "Klare Suppe mit Grießnockerl",
            &["A", "C", "G"],
        ),
        (
            "Tomatencremesuppe",
            "mit Basilikum und Croutons",
            &["A", "G"],
        ),
        ("Kartoffelsuppe", "mit Einlage und Brot", &["A", "G", "L"]),
        (
            "Caesar Salad",
            "mit Hühnerstreifen und Parmesan",
            &["A", "C", "G"],
        ),
        (
            "Leberknödelsuppe",
            "Klare Rindsuppe mit Leberknödel",
            &["A", "C", "G"],
        ),
        ("Gemüsesuppe", "mit frischem Saisongemüse", &["A", "L"]),
        ("Blattsalat", "mit Kernöl-Dressing und Kürbiskernen", &["H"]),
    ],
};

/// §5.2 — 40 items (10 weekdays × 4 pools). Deterministic per calendar day via the §5.1 LCG.
pub fn generate_demo_menus(clock: &dyn Clock) -> Vec<MenuItem> {
    let now = clock.now_epoch_ms();
    let today = Local
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch")
        .date_naive();

    // seed = year*10000 + (0-based month + 1)*100 + day  (chrono month() is already 1-based).
    let seed = today.year() as i64 * 10000 + today.month() as i64 * 100 + today.day() as i64;

    // Monday of the current week (Sunday belongs to the week that started 6 days earlier).
    let dow = today.weekday().num_days_from_sunday() as i64; // 0=Sun..6=Sat
    let diff = if dow == 0 { -6 } else { 1 - dow };
    let monday = today + Duration::days(diff);

    // the next 10 weekdays (Mon–Fri, skipping Sat/Sun) from that Monday.
    let mut days = Vec::with_capacity(10);
    let mut d = monday;
    while days.len() < 10 {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            days.push(d);
        }
        d += Duration::days(1);
    }

    let pools = [&MENU1, &MENU2, &MENU3, &SOUP_SALAD];
    let mut rng = Lcg::new(seed);
    let mut items = Vec::with_capacity(40);
    for (day_index, date) in days.iter().enumerate() {
        // one draw per pool, IN ORDER (Menu1, Menu2, Menu3, Soup & Salad).
        for pool in pools {
            let r = rng.next_f64();
            let dish_index = (day_index + (r * 3.0).floor() as usize) % 10;
            let (title, subtitle, allergens) = pool.dishes[dish_index];
            items.push(MenuItem {
                id: format!("{}-{}", pool.id_prefix, day_index),
                day: format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
                title: pool.category.display().to_string(),
                subtitle: format!("{title} {subtitle}"),
                allergens: allergens.iter().map(|s| s.to_string()).collect(),
                available: true,
                ordered: false,
                category: pool.category,
                price: pool.price.to_string(),
            });
        }
    }
    items
}

/// §5.3 — Gourmet demo bills for `check_last_month_number` ("0"/"1"/"2"; non-numeric → 0).
pub fn generate_demo_billings(clock: &dyn Clock, check_last_month_number: &str) -> Vec<Bill> {
    let offset: i64 = check_last_month_number.parse().unwrap_or(0);
    let today = local_today(clock);

    // target month = current month − offset (year underflow normalized).
    let total = today.year() as i64 * 12 + today.month0() as i64 - offset;
    let target_year = total.div_euclid(12) as i32;
    let target_month0 = total.rem_euclid(12) as u32; // 0-based

    // seed per target month = year*100 + 0-based month index (§5.1).
    let seed = target_year as i64 * 100 + target_month0 as i64;
    let mut rng = Lcg::new(seed);

    // weekdays of the target month that are on-or-before today.
    let bill_days = weekdays_of_month(target_year, target_month0 + 1)
        .into_iter()
        .filter(|d| *d <= today)
        .collect::<Vec<_>>();

    bill_days
        .iter()
        .enumerate()
        .map(|(i, date)| {
            let desc_index = (rng.next_f64() * 4.0).floor() as usize; // draw 1
            let price_index = (rng.next_f64() * 10.0).floor() as usize; // draw 2
            let total = BILLING_PRICES[price_index];
            Bill {
                bill_nr: 100_000 + i as i64,
                bill_date_epoch_ms: local_midnight_ms(*date),
                location: "Betriebsrestaurant".to_string(),
                items: vec![BillingItem {
                    id: format!("demo-bill-item-{i}"),
                    article_id: format!("demo-art-{desc_index}"),
                    count: 1,
                    description: BILLING_DESCRIPTIONS[desc_index].to_string(),
                    total,
                    subsidy: 1.50,
                    discount_value: 0.0,
                    is_custom_menu: false,
                }],
                billing: total - 1.50,
            }
        })
        .collect()
}

/// §5.4 — Ventopay demo transactions over [from_date_key, to_date_key], weekdays only.
/// Seeded on the CURRENT month (the requested range does not affect the seed).
pub fn generate_demo_transactions(
    clock: &dyn Clock,
    from_date_key: &str,
    to_date_key: &str,
) -> Vec<VentopayTransaction> {
    let (from, to) = match (
        NaiveDate::parse_from_str(from_date_key, "%Y-%m-%d"),
        NaiveDate::parse_from_str(to_date_key, "%Y-%m-%d"),
    ) {
        (Ok(f), Ok(t)) => (f, t),
        _ => return vec![],
    };
    let today = local_today(clock);
    let seed = today.year() as i64 * 100 + today.month0() as i64;
    let mut rng = Lcg::new(seed);

    let mut out = Vec::new();
    let mut counter = 0i64;
    let mut d = from;
    while d <= to {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            let roll = rng.next_f64(); // one draw per weekday
            if roll < 0.4 {
                let amount = TX_AMOUNTS[(rng.next_f64() * 6.0).floor() as usize];
                let hour = 7 + (rng.next_f64() * 4.0).floor() as u32; // 07–10
                let minute = (rng.next_f64() * 60.0).floor() as u32;
                out.push(VentopayTransaction {
                    id: format!("demo-vp-{counter}"),
                    date_epoch_ms: local_datetime_ms(d, hour, minute),
                    amount,
                    restaurant: "Kaffeeautomat".to_string(),
                    location: "Kaffeeautomat EG".to_string(),
                });
                counter += 1;
            }
        }
        d += Duration::days(1);
    }
    out
}

fn local_today(clock: &dyn Clock) -> NaiveDate {
    Local
        .timestamp_millis_opt(clock.now_epoch_ms())
        .single()
        .expect("valid epoch")
        .date_naive()
}

fn weekdays_of_month(year: i32, month: u32) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut d = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    while d.month() == month {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(d);
        }
        d += Duration::days(1);
    }
    out
}

fn local_midnight_ms(date: NaiveDate) -> i64 {
    local_datetime_ms(date, 0, 0)
}

fn local_datetime_ms(date: NaiveDate, hour: u32, minute: u32) -> i64 {
    let ndt = date.and_hms_opt(hour, minute, 0).expect("valid time");
    Local
        .from_local_datetime(&ndt)
        .single()
        .expect("valid local datetime")
        .timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    fn clock_on(y: i32, mo: u32, d: u32) -> FixedClock {
        let ms = Local
            .with_ymd_and_hms(y, mo, d, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        FixedClock { epoch_ms: ms }
    }

    #[test]
    fn produces_40_items_all_available_no_weekends() {
        let items = generate_demo_menus(&clock_on(2026, 2, 10)); // a Tuesday
        assert_eq!(items.len(), 40);
        assert!(items.iter().all(|i| i.available && !i.ordered));
        // all four categories present.
        for c in [
            MenuCategory::Menu1,
            MenuCategory::Menu2,
            MenuCategory::Menu3,
            MenuCategory::SoupAndSalad,
        ] {
            assert!(items.iter().any(|i| i.category == c));
        }
        // no Saturday/Sunday days.
        for i in &items {
            let d = chrono::NaiveDate::parse_from_str(&i.day, "%Y-%m-%d").unwrap();
            assert!(!matches!(d.weekday(), Weekday::Sat | Weekday::Sun));
        }
        // id shape + title carries the category display string.
        assert!(items.iter().any(|i| i.id == "demo-m1-0"));
        assert!(items.iter().any(|i| i.title == "MENÜ I"));
        assert!(items.iter().any(|i| i.price == "2,50 €")); // soup & salad price
    }

    #[test]
    fn deterministic_within_a_day() {
        let a = generate_demo_menus(&clock_on(2026, 2, 10));
        let b = generate_demo_menus(&clock_on(2026, 2, 10));
        assert_eq!(a, b);
    }

    #[test]
    fn ten_distinct_weekdays_starting_monday() {
        let items = generate_demo_menus(&clock_on(2026, 2, 11)); // a Wednesday
        let mut days: Vec<String> = items.iter().map(|i| i.day.clone()).collect();
        days.sort();
        days.dedup();
        assert_eq!(days.len(), 10);
        // the earliest day is the Monday of that week (2026-02-09).
        assert_eq!(days[0], "2026-02-09");
    }

    #[test]
    fn billing_past_month_has_all_weekdays_and_is_deterministic() {
        // from 2026-02-10, offset 1 → January 2026 (a full past month).
        let bills = generate_demo_billings(&clock_on(2026, 2, 10), "1");
        // January 2026 has 22 weekdays.
        assert_eq!(bills.len(), 22);
        assert_eq!(bills[0].bill_nr, 100_000);
        assert_eq!(bills[0].location, "Betriebsrestaurant");
        let item = &bills[0].items[0];
        assert_eq!(item.count, 1);
        assert_eq!(item.subsidy, 1.50);
        assert!((bills[0].billing - (item.total - 1.50)).abs() < 1e-9);
        assert!(BILLING_PRICES.contains(&item.total));
        assert!(BILLING_DESCRIPTIONS.contains(&item.description.as_str()));
        // deterministic
        assert_eq!(bills, generate_demo_billings(&clock_on(2026, 2, 10), "1"));
    }

    #[test]
    fn billing_non_numeric_offset_is_current_month() {
        let a = generate_demo_billings(&clock_on(2026, 2, 10), "garbage");
        let b = generate_demo_billings(&clock_on(2026, 2, 10), "0");
        assert_eq!(a, b);
    }

    #[test]
    fn transactions_deterministic_weekdays_only_kaffeeautomat() {
        let txs = generate_demo_transactions(&clock_on(2026, 2, 15), "2026-02-01", "2026-02-28");
        // ~40% of weekdays; at least some emitted, all Kaffeeautomat, ids sequential.
        assert!(!txs.is_empty());
        for (i, t) in txs.iter().enumerate() {
            assert_eq!(t.id, format!("demo-vp-{i}"));
            assert_eq!(t.restaurant, "Kaffeeautomat");
            assert_eq!(t.location, "Kaffeeautomat EG");
            assert!(TX_AMOUNTS.contains(&t.amount));
        }
        // deterministic within the same current month.
        assert_eq!(
            txs,
            generate_demo_transactions(&clock_on(2026, 2, 15), "2026-02-01", "2026-02-28")
        );
    }
}
