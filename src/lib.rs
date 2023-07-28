#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::contract]
pub trait LimitOrderContract {
    #[init]
    fn init(&self) {}

    #[endpoint]
    #[payable("*")]
    fn create_order(&self, price: BigUint) -> usize {
        let caller: ManagedAddress = self.blockchain().get_caller();
        let payment: EgldOrEsdtTokenPayment = self.call_value().egld_or_single_esdt();
        self.orders().push(&Order {
            owner: caller,
            payment,
            price,
            active: true,
        });
        self.orders().len()
    }

    #[endpoint]
    fn cancel_order(&self, order_id: usize) {
        let caller: ManagedAddress = self.blockchain().get_caller();
        let mut order: Order<Self::Api> = self.orders().get(order_id);
        assert!(order.owner == caller, "Only the owner can cancel the order");
        assert!(order.active, "The order is already cancelled");
        order.active = false;
        self.orders().set(order_id, &order);
        self.send().direct(
            &caller,
            &order.payment.token_identifier,
            order.payment.token_nonce,
            &order.payment.amount,
        );
    }

    #[endpoint]
    #[payable("EGLD")]
    fn fill_order(&self, order_id: usize) {
        let mut order: Order<Self::Api> = self.orders().get(order_id);
        assert!(order.active, "The order has been cancelled");
        let paid_amount = self.call_value().egld_value().clone_value();
        assert!(
            paid_amount >= order.price,
            "Not enough amount to fill the order"
        );
        let caller = self.blockchain().get_caller();
        self.send().direct(
            &caller,
            &order.payment.token_identifier,
            order.payment.token_nonce,
            &order.payment.amount,
        );
        self.send().direct(
            &order.owner,
            &EgldOrEsdtTokenIdentifier::egld(),
            0,
            &paid_amount,
        );
        order.active = false;
        self.orders().set(order_id, &order);
    }

    #[storage_mapper("orders")]
    fn orders(&self) -> VecMapper<Order<Self::Api>>;
}

#[derive(TopEncode, TopDecode, Debug)]
pub struct Order<M: ManagedTypeApi> {
    owner: ManagedAddress<M>,
    payment: EgldOrEsdtTokenPayment<M>,
    price: BigUint<M>,
    active: bool,
}
