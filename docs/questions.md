# Business Logic Questions Log

### Pickup slot prep-time enforcement
**Question**: Should prep-time filtering be based on a fixed system value or derived from the actual items in the cart?  
**My Understanding**: The prompt requires that items with prep requirements block too-soon slots. This should be cart-derived.  
**Solution**: The backend computes the maximum prep time across all cart items' SKU metadata and uses that value when filtering available slots and re-validating at checkout. The frontend passes the cart-derived prep time in the slot query.

### Voucher code storage security
**Question**: Should voucher codes be stored in plaintext or hashed/encrypted?  
**My Understanding**: Voucher codes are sensitive — storing plaintext enables enumeration attacks. They should be stored as a hash for lookup and encrypted for display.  
**Solution**: Voucher codes are SHA-256 hashed before storage in the `vouchers.code` and `reservations.voucher_code` columns (VARCHAR(64)). The original code is only held in memory during the request lifecycle.

### Session cookie vs JWT
**Question**: The prompt specifies rotating HMAC-signed session cookies as the primary auth mechanism. Should JWT be supported at all?  
**My Understanding**: JWT bearer tokens could be useful for API clients, but they add complexity and a second auth surface to maintain.  
**Solution**: The auth guard uses HMAC-signed session cookies exclusively. API clients (e.g. WASM frontend) receive the signed cookie value in the login response body and attach it via the `Cookie` header manually. This keeps a single auth path with consistent security properties (idle timeout, rotation, server-side revocation).

### Reservation lock quantity handling
**Question**: When a customer adds quantity > 1 of an item to the cart, should the reservation lock decrement stock by the full quantity?  
**My Understanding**: Yes — the lock must reserve the exact quantity requested to prevent oversell.  
**Solution**: The lock acquisition passes the cart item quantity to the reservation service, which validates `stock >= requested_quantity` and decrements by that amount. The lock row stores the reserved quantity for release on cancellation.

### Order cancellation permissions
**Question**: Can customers cancel orders at any stage, or only before preparation begins?  
**My Understanding**: Customers should only cancel before the order is accepted. After acceptance, only staff/admin can cancel (with a reason).  
**Solution**: The cancellation endpoint checks the caller's role against the current order status. Customers can cancel `Pending` orders only. Staff/Admin can cancel up to `Preparing` with a mandatory reason field.

### Training exam retake policy
**Question**: Can staff retake failed exams immediately, or is there a cooldown?  
**My Understanding**: A cooldown prevents gaming the system. A 24-hour cooldown between attempts on the same exam is reasonable.  
**Solution**: The exam attempt service checks the timestamp of the most recent attempt for the same exam/user pair and rejects new attempts within 24 hours of a failed attempt.
