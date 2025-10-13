use orderbook_rust::*;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OrderBook Rust - Advanced Demo");
    println!("================================\n");
    
    let mut order_book = OrderBook::new();
    
    // demo 1: multiple order types
    println!("Demo 1: Multiple Order Types");
    println!("-------------------------------");
    
    // add a limit buy order
    let buy_order = Order::new(
        Side::Buy,
        OrderType::Limit,
        100_000, // $100.00
        1000,
        Some("trader1".to_string()),
    );
    let trades = order_book.add_order(buy_order)?;
    println!("Limit Buy: 1000 shares at $100.00 - Trades: {}", trades.len());
    
    // add a limit sell order
    let sell_order = Order::new(
        Side::Sell,
        OrderType::Limit,
        101_000, // $101.00
        500,
        Some("trader2".to_string()),
    );
    let trades = order_book.add_order(sell_order)?;
    println!("Limit Sell: 500 shares at $101.00 - Trades: {}", trades.len());
    
    // add a market buy order (should match with the sell order)
    let market_buy = Order::new(
        Side::Buy,
        OrderType::Market,
        0, // price doesn't matter for market orders
        300,
        Some("trader3".to_string()),
    );
    let trades = order_book.add_order(market_buy)?;
    println!("Market Buy: 300 shares - Trades: {}", trades.len());
    if !trades.is_empty() {
        println!("   Trade executed: {} shares at ${:.2}", 
                 trades[0].quantity, 
                 trades[0].price as f64 / 1_000_000.0);
    }
    
    // demo 2: fill or kill order
    println!("\nDemo 2: Fill or Kill Order");
    println!("-----------------------------");
    
    // add a fok order for more than available
    let fok_order = Order::new(
        Side::Buy,
        OrderType::FillOrKill,
        101_000,
        1000, // more than available (200 shares remaining)
        Some("trader4".to_string()),
    );
    let trades = order_book.add_order(fok_order)?;
    println!("FOK Order: 1000 shares at $101.00 - Trades: {} (rejected)", trades.len());
    
    // add a fok order for available quantity
    let fok_order2 = Order::new(
        Side::Buy,
        OrderType::FillOrKill,
        101_000,
        200, // exactly what's available
        Some("trader5".to_string()),
    );
    let trades = order_book.add_order(fok_order2)?;
    println!("FOK Order: 200 shares at $101.00 - Trades: {} (filled)", trades.len());
    
    // demo 3: Immediate or Cancel order
    println!("\nDemo 3: Immediate or Cancel Order");
    println!("------------------------------------");
    
    // add a sell order first
    let sell_order2 = Order::new(
        Side::Sell,
        OrderType::Limit,
        99_000, // $99.00
        800,
        Some("trader6".to_string()),
    );
    let trades = order_book.add_order(sell_order2)?;
    println!("Limit Sell: 800 shares at $99.00 - Trades: {}", trades.len());
    
    // add an IOC order for more than available
    let ioc_order = Order::new(
        Side::Buy,
        OrderType::ImmediateOrCancel,
        100_000,
        1000, // more than available (800 shares)
        Some("trader7".to_string()),
    );
    let trades = order_book.add_order(ioc_order)?;
    println!("IOC Order: 1000 shares at $100.00 - Trades: {} (partial fill)", trades.len());
    if !trades.is_empty() {
        println!("   Trade executed: {} shares at ${:.2}", 
                 trades[0].quantity, 
                 trades[0].price as f64 / 1_000_000.0);
    }
    
    // demo 4: Order Book Snapshot
    println!("\nDemo 4: Order Book Snapshot");
    println!("------------------------------");
    
    let snapshot = order_book.get_order_book_snapshot(10);
    println!("Current Order Book:");
    println!("Bids ({} levels):", snapshot.bids.len());
    for (i, bid) in snapshot.bids.iter().enumerate() {
        println!("  {}. ${:.2} - {} shares ({} orders)", 
                 i + 1,
                 bid.price as f64 / 1_000_000.0, 
                 bid.quantity, 
                 bid.order_count);
    }
    
    println!("Asks ({} levels):", snapshot.asks.len());
    for (i, ask) in snapshot.asks.iter().enumerate() {
        println!("  {}. ${:.2} - {} shares ({} orders)", 
                 i + 1,
                 ask.price as f64 / 1_000_000.0, 
                 ask.quantity, 
                 ask.order_count);
    }
    
    // demo 5: Best Bid/Ask
    println!("\nDemo 5: Best Bid/Ask");
    println!("----------------------");
    
    if let Some(best_bid) = order_book.get_best_bid() {
        println!("Best Bid: ${:.2}", best_bid as f64 / 1_000_000.0);
    } else {
        println!("Best Bid: None");
    }
    
    if let Some(best_ask) = order_book.get_best_ask() {
        println!("Best Ask: ${:.2}", best_ask as f64 / 1_000_000.0);
    } else {
        println!("Best Ask: None");
    }
    
    // demo 6: Statistics
    println!("\nDemo 6: Performance Statistics");
    println!("--------------------------------");
    
    let stats = order_book.get_market_data_stats();
    println!("OrderBook Statistics:");
    println!("  • Orders processed: {}", stats.new_orders);
    println!("  • Cancellations: {}", stats.cancellations);
    println!("  • Modifications: {}", stats.modifications);
    println!("  • Trades executed: {}", stats.trades);
    println!("  • Average latency: {:.2} μs", stats.get_average_latency_micros());
    println!("  • Max latency: {:.2} μs", stats.max_latency.as_micros());
    println!("  • Min latency: {:.2} μs", stats.min_latency.as_micros());
    
    // demo 7: Order cancellation
    println!("\nDemo 7: Order Cancellation");
    println!("-----------------------------");
    
    // add an order to cancel
    let order_to_cancel = Order::new(
        Side::Buy,
        OrderType::Limit,
        95_000, // $95.00
        500,
        Some("trader8".to_string()),
    );
    let order_id = order_to_cancel.id;
    let trades = order_book.add_order(order_to_cancel)?;
    println!("Added order to cancel: {} shares at $95.00 - Trades: {}", 500, trades.len());
    
    // cancel the order
    order_book.cancel_order(order_id)?;
    println!("Order cancelled successfully");
    
    // verify it's gone
    let snapshot = order_book.get_order_book_snapshot(5);
    let has_order = snapshot.bids.iter().any(|bid| bid.price == 95_000);
    println!("Verification: Order removed from book: {}", !has_order);
    
    println!("\nAdvanced Demo Completed Successfully!");
    println!("=====================================");
    
    Ok(())
}
