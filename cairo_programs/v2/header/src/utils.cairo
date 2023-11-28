
mod print {
    use debug::PrintTrait;

    impl ArrayPrintImpl of PrintTrait<Array<u8>> {
        fn print(self: Array<u8>) {
            let mut i = 0;
            let length = self.len();
            'Array<u8>['.print();
            loop {
                if i >= length {
                    break;
                }
                let byte: u8 = *(self.at(i));
                byte.print();
                i = i + 1;
            };
            ']____'.print();
        }
    }
}