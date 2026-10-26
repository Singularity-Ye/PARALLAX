/**
 * Vanilla TearTicket (adapted from React Bits Micro/TearTicket)
 * Interactive paper ticket with authentic perforated edge and physical tear effect.
 */
export function initTearTicket(selector = '.paper-ticket, .hero-note') {
  if (typeof window === 'undefined') return;

  const tickets = document.querySelectorAll(selector);

  tickets.forEach(ticket => {
    if (ticket.dataset.ticketInitialized) return;
    ticket.dataset.ticketInitialized = 'true';
    ticket.classList.add('paper-ticket');

    // Add sawtooth perforated bottom bar & tear stub
    const stub = document.createElement('div');
    stub.className = 'ticket-stub';
    stub.innerHTML = `
      <div class="ticket-perforation" aria-hidden="true">
        <span class="perf-line"></span>
        <span class="perf-notch notch-left"></span>
        <span class="perf-notch notch-right"></span>
      </div>
      <div class="stub-bar">
        <span class="stub-code">PARALLAX // REC-01</span>
        <button type="button" class="stub-tear-btn" title="撕下收执">
          <span class="tear-icon">✂</span> 撕下收执
        </button>
      </div>
    `;

    ticket.appendChild(stub);

    const tearBtn = stub.querySelector('.stub-tear-btn');
    tearBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      if (ticket.dataset.torn === 'true') {
        // Toggle restore
        ticket.dataset.torn = 'false';
        stub.style.transform = '';
        stub.style.opacity = '1';
        tearBtn.innerHTML = '<span class="tear-icon">✂</span> 撕下收执';
      } else {
        // Physical paper tear animation
        ticket.dataset.torn = 'true';
        stub.style.transition = 'transform 0.45s cubic-bezier(0.25, 1, 0.5, 1), opacity 0.35s ease';
        stub.style.transform = 'translate(6px, 18px) rotate(3.5deg)';
        stub.style.opacity = '0.85';
        tearBtn.innerHTML = '<span class="tear-icon">✦</span> 已留存收执（点击接回）';

        // Custom event for audio/haptics if needed
        ticket.dispatchEvent(new CustomEvent('ticket:torn', { bubbles: true }));
      }
    });
  });
}
