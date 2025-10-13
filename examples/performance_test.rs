use orderbook_rust::*;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OrderBook Rust - Performance Test");
    println!("===================================\n");
    
    let mut order_book = OrderBook::new();
    
    // test 1: Add many orders
    println!("Test 1: Adding 10,000 Orders");
    println!("-------------------------------");
    
    let start = Instant::now();
    for i in 0..10_000 {
        let price = 100_000 + (i % 1000) as u64; // prices from $100.00 to $100.99
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        let quantity = 100 + (i % 900) as u64; // quantities from 100 to 999
        
        let order = Order::new(
            side,
            OrderType::Limit,
            price,
            quantity,
            Some(format!("trader_{}", i)),
        );
        
        order_book.add_order(order)?;
    }
    let duration = start.elapsed();
    
    println!("Added 10,000 orders in {:.2}ms", duration.as_millis());
    println!("   Rate: {:.0} orders/second", 10_000.0 / duration.as_secs_f64());
    
    // test 2: Get snapshots
    println!("\nTest 2: Getting 1,000 Snapshots");
    println!("----------------------------------");
    
    let start = Instant::now();
    for _ in 0..1_000 {
        let _snapshot = order_book.get_order_book_snapshot(10);
    }
    let duration = start.elapsed();
    
    println!("Got 1,000 snapshots in {:.2}ms", duration.as_millis());
    println!("   Rate: {:.0} snapshots/second", 1_000.0 / duration.as_secs_f64());
    
    // test 3: Cancel orders (simplified)
    println!("\nTest 3: Cancelling Orders");
    println!("-----------------------------");
    
    // get some existing order ids from the order book
    let _snapshot = order_book.get_order_book_snapshot(100);
    let mut order_ids = Vec::new();
    
    // add a few specific orders to cancel
    for i in 0..100 {
        let order = Order::new(
            Side::Buy,
            OrderType::Limit,
            500_000 + i as u64, // very high prices to avoid matching
            100,
            Some(format!("cancel_trader_{}", i)),
        );
        order_ids.push(order.id);
        order_book.add_order(order)?;
    }
    
    println!("   Added {} orders for cancellation", order_ids.len());
    
    let order_count = order_ids.len();
    let start = Instant::now();
    for order_id in order_ids {
        order_book.cancel_order(order_id)?;
    }
    let duration = start.elapsed();
    
    println!("Cancelled {} orders in {:.2}ms", order_count, duration.as_millis());
    println!("   Rate: {:.0} cancellations/second", order_count as f64 / duration.as_secs_f64());
    
    // test 4: Matching performance
    println!("\nTest 4: Matching Performance");
    println!("-------------------------------");
    
    // add some orders that will match
    let start = Instant::now();
    let mut total_trades = 0;
    
    for i in 0..5_000 {
        let price = 150_000 + (i % 100) as u64; // prices from $150.00 to $150.99
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        let quantity = 50 + (i % 50) as u64; // quantities from 50 to 99
        
        let order = Order::new(
            side,
            OrderType::Limit,
            price,
            quantity,
            Some(format!("match_trader_{}", i)),
        );
        
        let trades = order_book.add_order(order)?;
        total_trades += trades.len();
    }
    let duration = start.elapsed();
    
    println!("Processed 5,000 orders with {} trades in {:.2}ms", 
             total_trades, duration.as_millis());
    println!("   Rate: {:.0} orders/second", 5_000.0 / duration.as_secs_f64());
    println!("   Trade rate: {:.0} trades/second", total_trades as f64 / duration.as_secs_f64());
    
    // final statistics
    println!("\nFinal Statistics");
    println!("------------------");
    
    let stats = order_book.get_market_data_stats();
    let snapshot = order_book.get_order_book_snapshot(5);
    
    println!("OrderBook State:");
    println!("  • Total orders processed: {}", stats.new_orders);
    println!("  • Total cancellations: {}", stats.cancellations);
    println!("  • Total trades: {}", stats.trades);
    println!("  • Current bid levels: {}", snapshot.bids.len());
    println!("  • Current ask levels: {}", snapshot.asks.len());
    println!("  • Average latency: {:.2} μs", stats.get_average_latency_micros());
    println!("  • Max latency: {:.2} μs", stats.max_latency.as_micros());
    
    if let Some(best_bid) = order_book.get_best_bid() {
        println!("  • Best bid: ${:.2}", best_bid as f64 / 1_000_000.0);
    }
    if let Some(best_ask) = order_book.get_best_ask() {
        println!("  • Best ask: ${:.2}", best_ask as f64 / 1_000_000.0);
    }
    
    println!("\nPerformance Test Completed Successfully!");
    println!("==========================================");
    
    Ok(())
}
