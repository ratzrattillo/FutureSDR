use futuresdr::anyhow::Result;
use futuresdr::macros::async_trait;
use futuresdr::num_complex::Complex32;
use futuresdr::runtime::Block;
use futuresdr::runtime::BlockMeta;
use futuresdr::runtime::BlockMetaBuilder;
use futuresdr::runtime::Kernel;
use futuresdr::runtime::MessageIo;
use futuresdr::runtime::MessageIoBuilder;
use futuresdr::runtime::StreamIo;
use futuresdr::runtime::StreamIoBuilder;
use futuresdr::runtime::WorkIo;
use std::collections::VecDeque;

struct ClockTrackingLoop {
    avg_period: f32,
    max_avg_period: f32,
    min_avg_period: f32,
    nom_avg_period: f32,
    inst_period: f32,
    phase: f32,
    zeta: f32,
    omega_n_norm: f32,
    ted_gain: f32,
    alpha: f32,
    beta: f32,
    prev_avg_period: f32,
    prev_inst_period: f32,
    prev_phase: f32,
}

impl ClockTrackingLoop {
    fn new(loop_bw: f32, max_period: f32, min_period: f32, nominal_period: f32, damping: f32, ted_gain: f32) -> Self {
        let mut s = Self {
            avg_period: nominal_period,
            max_avg_period: max_period,
            min_avg_period: min_period,
            nom_avg_period: nominal_period,
            inst_period: nominal_period,
            phase: 0.0,
            zeta: damping,
            omega_n_norm: loop_bw,
            ted_gain,
            alpha: 0.0,
            beta: 0.0,
            prev_avg_period: nominal_period,
            prev_inst_period: nominal_period,
            prev_phase: 0.0,
        }

        s.set_max_avg_period(max_period);
        s.set_min_avg_period(min_period);
        s.set_nom_avg_period(nominal_period);

        s.set_avg_period(self.nom_avg_period);
        s.set_inst_period(self.nom_avg_period);

        if s.zeta < 0.0 {
            panic!("clock_tracking_loop: damping factor must be > 0.0");
        }

        if s.omega_n_norm < 0.0 {
            panic!("clock_tracking_loop: loop bandwidth must be greater than 0.0");
        }

        if s.ted_gain <= 0.0 {
            panic!( "clock_tracking_loop: expected ted gain must be greater than 0.0");
        }
        s.update_gains();
        s
    }

    fn advance_loop(&mut self, error: f32) {
        self.prev_avg_period = self.avg_period;
        self.prev_inst_period = self.inst_period;
        self.prev_phase = self.phase;
        self.avg_period = self.avg_period + self.beta * error;
        period_limit();

        self.inst_period = self.avg_period + self.alpha * error;
        if self.inst_period <= 0.0 {
            self.inst_period = self.avg_period;
        }

        self.phase = self.phase + self.inst_period;
    }

    fn revert_loop(&mut self) {
        self.avg_period = self.prev_avg_period;
        self.inst_period = self.prev_inst_period;
        self.phase = self.prev_phase;
    }

    fn phase_wrap(&mut self) {
        let period = self.avg_period;
        let limit = period / 2.0;

        while (self.phase > limit) {
            self.phase -= period;
        }

        while (self.phase <= -limit) {
            self.phase += period;
        }
    }

    fn period_limit(&mut self) {
        if self.avg_period > self.max_avg_period {
            self.avg_period = self.max_avg_period;
        } else if self.avg_period < self.min_avg_period {
            self.avg_period = self.min_avg_period;
        }
    }

    fn update_gains(&mut self) {
        let omega_n_T;
        let omega_d_T;
        let zeta_omega_n_T;
        let cosx_omega_d_T;

        let k0;
        let k1;
        let sinh_zeta_omega_n_T;
        let alpha;
        let beta;

        omega_n_T = self.omega_n_norm;
        zeta_omega_n_T = self.zeta * omega_n_T;
        k0 = 2.0 / self.ted_gain;
        k1 = (-zeta_omega_n_T).exp();
        sinh_zeta_omega_n_T = (zeta_omega_n_T).sinh();

        if self.zeta > 1.0 {
            omega_d_T = omega_n_T * (self.zeta * self.zeta - 1.0).sqrt();
            cosx_omega_d_T = omega_d_T.cosh();

        } else if self.zeta == 1.0 {
            omega_d_T = 0.0;
            cosx_omega_d_T = 1.0;

        } else {
            omega_d_T = omega_n_T * (1.0 - self.zeta * self.zeta).sqrt();
            cosx_omega_d_T = omega_d_T.cos();
        }

        alpha = k0 * k1 * sinh_zeta_omega_n_T;
        beta = k0 * (1.0 - k1 * (sinh_zeta_omega_n_T + cosx_omega_d_T));

        set_alpha(alpha);
        set_beta(beta);
    }

    fn set_loop_bandwidth(&mut self, bw: f32) {
        assert!(bw >= 0.0);
        self.omega_n_norm = bw;
        update_gains();
    }

    fn set_damping_factor(&mut self, df: f32) {
        assert!(df >= 0.0);
        self.zeta = df;
        update_gains();
    }

    fn set_ted_gain(&mut self, ted_gain: f32) {
        assert!(ted_gain > 0.0);
        self.ted_gain = ted_gain;
        update_gains();
    }

    fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    fn set_beta(&mut self, beta: f32) {
        self.beta = beta;
    }

void clock_tracking_loop::set_avg_period(float period)
{
    d_avg_period = period;
    d_prev_avg_period = period;
}

void clock_tracking_loop::set_inst_period(float period)
{
    d_inst_period = period;
    d_prev_inst_period = period;
}

void clock_tracking_loop::set_phase(float phase)
{
    // This previous phase is likely inconsistent with the tracking,
    // but if the caller is setting the phase, the odds of
    // revert_loop() being called are slim.
    d_prev_phase = phase;

    d_phase = phase;
}

void clock_tracking_loop::set_max_avg_period(float period) { d_max_avg_period = period; }

void clock_tracking_loop::set_min_avg_period(float period) { d_min_avg_period = period; }

void clock_tracking_loop::set_nom_avg_period(float period)
{
    if (period < d_min_avg_period || period > d_max_avg_period) {
        d_nom_avg_period = (d_max_avg_period + d_min_avg_period) / 2.0f;
    } else {
        d_nom_avg_period = period;
    }
}

/*******************************************************************
 * GET FUNCTIONS
 *******************************************************************/

float clock_tracking_loop::get_loop_bandwidth() const { return d_omega_n_norm; }

float clock_tracking_loop::get_damping_factor() const { return d_zeta; }

float clock_tracking_loop::get_ted_gain() const { return d_ted_gain; }

float clock_tracking_loop::get_alpha() const { return d_alpha; }

float clock_tracking_loop::get_beta() const { return d_beta; }

float clock_tracking_loop::get_avg_period() const { return d_avg_period; }

float clock_tracking_loop::get_inst_period() const { return d_inst_period; }

float clock_tracking_loop::get_phase() const { return d_phase; }

float clock_tracking_loop::get_max_avg_period() const { return d_max_avg_period; }

float clock_tracking_loop::get_min_avg_period() const { return d_min_avg_period; }

float clock_tracking_loop::get_nom_avg_period() const { return d_nom_avg_period; }


}

struct TimingErrorDetector {
    input: VecDeque<Complex32>,
    error: f32,
    error_depth: usize,
    input_clock: i32,
    inputs_per_symbol: usize,
    needs_derivative: bool,
    needs_lookahead: bool,
    prev_error: f32,
}

impl TimingErrorDetector {

    fn new(inputs_per_symbol: usize, error_depth: usize, needs_lookahead: bool, needs_derivative: bool) -> Self {
        let mut s = Self {
            error: 0.0,
            error_depth,
            input: VecDeque::new(),
            input_clock: 0,
            inputs_per_symbol,
            needs_derivative,
            needs_lookahead,
            prev_error: 0.0,
        };
        s.sync_reset();
        s
    }

    fn inputs_per_symbol(&self) -> usize {
        self.inputs_per_symbol
    }

    fn input(&mut self, x: f32, dx: f32) {
        self.input.push_front(Complex32::new(x, 0.0));
        self.input.pop_back();
        assert_eq!(self.needs_derivative, false);
        assert_eq!(self.needs_lookahead, false);


        self.advance_input_clock();
        if self.input_clock == 0 {
            self.prev_error = self.error;
            self.error = self.compute_error();
        }
    }

    fn needs_lookahead(&self) -> bool {
        self.needs_lookahead
    }

    fn input_lookahead(&mut self, x: f32, dx: f32) {
        assert_eq!(self.needs_lookahead, false);
        // do not need lookahead
    }

    fn needs_derivative(&self) -> bool {
        self.needs_derivative
    }

    fn error(&self) -> f32 {
        self.error
    }

    fn revert(&mut self, preserve_error: bool) {
        if (self.input_clock == 0) && (preserve_error == false) {
            self.error = self.prev_error; 
        }
        self.revert_input_clock();

        assert_eq!(self.needs_derivative, false);

        self.input.push_back(*self.input.back().unwrap());
        self.input.pop_front();
    }

    fn sync_reset(&mut self) {
        self.error = 0.0;
        self.prev_error = 0.0;

        self.input = VecDeque::from_iter(vec![Complex32::new(0.0, 0.0); self.error_depth].into_iter());
        self.sync_reset_input_clock();
    }

    fn advance_input_clock(&mut self) {
        self.input_clock = (self.input_clock + 1) % self.inputs_per_symbol as i32;
    }

    fn revert_input_clock(&mut self)
    {
        if self.input_clock == 0 {
            self.input_clock = self.inputs_per_symbol as i32 - 1;
        } else {
            self.input_clock -= 1;
        }
    }

    fn sync_reset_input_clock(&mut self) {
        self.input_clock = self.inputs_per_symbol as i32 - 1;
    }

    fn compute_error(&self) -> f32 {
        (self.input[2].re - self.input[0].re) * self.input[1].re
    }
}

pub struct SymbolSync {
    ted: TimingErrorDetector,
}

impl SymbolSync {
    pub fn new() -> Block {
        Block::new(
            BlockMetaBuilder::new("SymbolSync").build(),
            StreamIoBuilder::new()
                .add_input::<f32>("in")
                .add_output::<f32>("out")
                .build(),
            MessageIoBuilder::new().build(),
            Self {
                ted: TimingErrorDetector::new(2, 3, false, false),
            },
        )
    }
}

#[async_trait]
impl Kernel for SymbolSync {
    async fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _m: &mut MessageIo<Self>,
        _b: &mut BlockMeta,
    ) -> Result<()> {
        let input = sio.input(0).slice::<f32>();
        let out = sio.output(0).slice::<f32>();

        io.finished = true;

        Ok(())
    }
}