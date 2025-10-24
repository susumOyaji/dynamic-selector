document.addEventListener('DOMContentLoaded', () => {
    const quoteForm = document.getElementById('quote-form');
    const responseEl = document.getElementById('api-response').querySelector('code');

    // When deployed, this might need to be an absolute URL to your worker.
    const API_BASE_URL = 'http://127.0.0.1:8787';

    const displayResult = (data) => {
        responseEl.textContent = JSON.stringify(data, null, 2);
    };

    const displayLoading = () => {
        responseEl.textContent = 'Loading...';
    };

    const displayError = (err) => {
        responseEl.textContent = `Error: ${err.message}\n\nCheck the browser console for more details. Make sure the worker is running.`;
        console.error(err);
    };

    // Handler for /quote
    quoteForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        displayLoading();
        const codes = document.getElementById('quote-codes').value;
        const url = new URL(`${API_BASE_URL}/quote`);
        url.searchParams.append('code', codes);

        try {
            const res = await fetch(url);
            if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
            const data = await res.json();
            displayResult(data);
        } catch (err) {
            displayError(err);
        }
    });
});
