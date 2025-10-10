// Fetch latest release from GitHub
async function fetchLatestRelease() {
    try {
        // For private repos, you can set a GitHub token in localStorage:
        // localStorage.setItem('github_token', 'ghp_yourtoken')
        const token = localStorage.getItem('github_token');
        const headers = token ? { 'Authorization': `token ${token}` } : {};

        const response = await fetch('https://api.github.com/repos/tnnl-to/tnnl/releases/latest', { headers });
        if (!response.ok) {
            throw new Error('Failed to fetch release');
        }
        const release = await response.json();

        // Update version display
        const versionElement = document.getElementById('macos-version');
        if (versionElement) {
            versionElement.textContent = release.tag_name || 'v1.0.0';
        }

        // Find platform-specific assets
        const macAsset = release.assets.find(asset =>
            asset.name.includes('darwin') ||
            asset.name.includes('macos') ||
            asset.name.includes('.dmg') ||
            asset.name.includes('aarch64-apple')
        );

        const windowsAsset = release.assets.find(asset =>
            asset.name.includes('windows') ||
            asset.name.includes('.exe') ||
            asset.name.includes('.msi') ||
            asset.name.includes('x86_64-pc-windows')
        );

        const linuxAsset = release.assets.find(asset =>
            asset.name.includes('linux') ||
            asset.name.includes('.AppImage') ||
            asset.name.includes('.deb') ||
            asset.name.includes('x86_64-unknown-linux')
        );

        // Update macOS download button
        const macButton = document.getElementById('macos-download');
        const macVersion = document.getElementById('macos-version');
        if (macAsset && macButton) {
            macButton.classList.remove('btn--coming-soon');
            macButton.disabled = false;
            macButton.onclick = () => {
                trackEvent('download_click', {
                    platform: 'macOS',
                    version: release.tag_name,
                    file: macAsset.name
                });
                window.location.href = macAsset.browser_download_url;
            };
            if (macVersion) macVersion.textContent = release.tag_name;
        } else {
            if (macVersion) macVersion.textContent = 'Coming Soon';
        }

        // Update Windows download button
        const winButton = document.getElementById('windows-download');
        const winVersion = document.getElementById('windows-version');
        if (windowsAsset && winButton) {
            winButton.classList.remove('btn--coming-soon');
            winButton.disabled = false;
            winButton.onclick = () => {
                trackEvent('download_click', {
                    platform: 'Windows',
                    version: release.tag_name,
                    file: windowsAsset.name
                });
                window.location.href = windowsAsset.browser_download_url;
            };
            if (winVersion) winVersion.textContent = release.tag_name;
        }

        // Update Linux download button
        const linuxButton = document.getElementById('linux-download');
        const linuxVersion = document.getElementById('linux-version');
        if (linuxAsset && linuxButton) {
            linuxButton.classList.remove('btn--coming-soon');
            linuxButton.disabled = false;
            linuxButton.onclick = () => {
                trackEvent('download_click', {
                    platform: 'Linux',
                    version: release.tag_name,
                    file: linuxAsset.name
                });
                window.location.href = linuxAsset.browser_download_url;
            };
            if (linuxVersion) linuxVersion.textContent = release.tag_name;
        }
    } catch (error) {
        console.error('Failed to fetch latest release:', error);
        // Fallback to default version
        const versionElement = document.getElementById('macos-version');
        if (versionElement) {
            versionElement.textContent = 'v1.0.0';
        }
    }
}

// Wait for DOM to be fully loaded
document.addEventListener('DOMContentLoaded', function() {
    // Fetch latest release info
    fetchLatestRelease();
    
    // Mobile Navigation Toggle
    const navToggle = document.getElementById('navToggle');
    const navMenu = document.getElementById('navMenu');

    if (navToggle && navMenu) {
        navToggle.addEventListener('click', () => {
            navMenu.classList.toggle('nav__menu--active');
            navToggle.classList.toggle('nav__toggle--active');
        });
    }

    // Close mobile menu when clicking on links
    const navLinks = document.querySelectorAll('.nav__link');
    navLinks.forEach(link => {
        link.addEventListener('click', () => {
            if (navMenu) {
                navMenu.classList.remove('nav__menu--active');
            }
            if (navToggle) {
                navToggle.classList.remove('nav__toggle--active');
            }
        });
    });

    // Smooth scrolling function
    function scrollToSection(sectionId) {
        const element = document.getElementById(sectionId);
        if (element) {
            const headerHeight = document.querySelector('.header') ? document.querySelector('.header').offsetHeight : 80;
            const elementPosition = element.offsetTop - headerHeight - 20;
            
            window.scrollTo({
                top: Math.max(0, elementPosition),
                behavior: 'smooth'
            });
        }
    }

    // Make scrollToSection available globally for onclick handlers
    window.scrollToSection = scrollToSection;

    // Handle smooth scrolling for navigation links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const targetId = this.getAttribute('href').substring(1);
            
            if (targetId) {
                scrollToSection(targetId);
            }
        });
    });

    // Pricing button interactions - Fixed
    const pricingButtons = document.querySelectorAll('.pricing-btn');
    pricingButtons.forEach(button => {
        button.addEventListener('click', (e) => {
            e.preventDefault();
            
            const planName = button.getAttribute('data-plan') || 'Unknown Plan';
            
            if (planName === 'Free') {
                alert('Redirecting to free download...');
                setTimeout(() => scrollToSection('download'), 500);
            } else if (planName === 'Custom Domain') {
                alert('Starting Custom Domain plan signup...');
            } else if (planName === 'Additional Tunnels') {
                alert('Starting Additional Tunnels plan signup...');
            } else {
                alert(`Starting ${planName} plan signup...`);
            }
        });
    });

    // Note: macOS download button is handled by fetchLatestRelease()
    // Windows and Linux buttons are disabled with coming-soon styling

    // Add click ripple effect to buttons
    function createRipple(event) {
        const button = event.currentTarget;
        const circle = document.createElement('span');
        const diameter = Math.max(button.clientWidth, button.clientHeight);
        const radius = diameter / 2;
        
        const rect = button.getBoundingClientRect();
        circle.style.width = circle.style.height = `${diameter}px`;
        circle.style.left = `${event.clientX - rect.left - radius}px`;
        circle.style.top = `${event.clientY - rect.top - radius}px`;
        circle.classList.add('ripple');
        
        const existingRipple = button.querySelector('.ripple');
        if (existingRipple) {
            existingRipple.remove();
        }
        
        button.appendChild(circle);
    }

    // Apply ripple effect to all buttons
    document.querySelectorAll('.btn').forEach(button => {
        button.addEventListener('click', createRipple);
    });

    // Track button clicks for analytics
    document.querySelectorAll('.btn').forEach(button => {
        button.addEventListener('click', () => {
            const buttonText = button.textContent.trim();
            const section = button.closest('section')?.className || 'unknown';
            trackEvent('button_click', {
                button_text: buttonText,
                section: section
            });
        });
    });

    // Smooth reveal animation for hero elements
    const heroElements = document.querySelectorAll('.hero__headline, .hero__subheadline, .hero__actions, .tunnel-visual');
    
    heroElements.forEach((element, index) => {
        setTimeout(() => {
            element.classList.add('animate-in');
        }, index * 200);
    });
});

// Header scroll effect
let lastScrollY = window.scrollY;
const header = document.querySelector('.header');

// Throttle function for performance
function throttle(func, limit) {
    let inThrottle;
    return function() {
        const args = arguments;
        const context = this;
        if (!inThrottle) {
            func.apply(context, args);
            inThrottle = true;
            setTimeout(() => inThrottle = false, limit);
        }
    }
}

const throttledScrollHandler = throttle(() => {
    const currentScrollY = window.scrollY;
    
    // Add/remove scrolled class for styling
    if (header) {
        if (currentScrollY > 50) {
            header.classList.add('header--scrolled');
        } else {
            header.classList.remove('header--scrolled');
        }
        
        // Hide/show header on scroll
        if (currentScrollY > lastScrollY && currentScrollY > 100) {
            header.style.transform = 'translateY(-100%)';
        } else {
            header.style.transform = 'translateY(0)';
        }
    }
    
    lastScrollY = currentScrollY;
}, 10);

window.addEventListener('scroll', throttledScrollHandler);

// Intersection Observer for fade-in animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.classList.add('animate-in');
        }
    });
}, observerOptions);

// Observe elements for animation
const animateElements = document.querySelectorAll('.value-prop, .step, .feature, .pricing-card, .download-card');
animateElements.forEach(el => {
    observer.observe(el);
});

// Keyboard navigation support
document.addEventListener('keydown', (e) => {
    // ESC key closes mobile menu
    if (e.key === 'Escape') {
        const navMenu = document.getElementById('navMenu');
        const navToggle = document.getElementById('navToggle');
        
        if (navMenu) {
            navMenu.classList.remove('nav__menu--active');
        }
        if (navToggle) {
            navToggle.classList.remove('nav__toggle--active');
        }
    }
});

// Track user interactions with Umami
function trackEvent(eventName, properties = {}) {
    // Send to Umami if available
    if (typeof umami !== 'undefined' && umami.track) {
        umami.track(eventName, properties);
    }

    // Also log to console in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Analytics Event:', eventName, properties);
    }
}

// Track section views
const sectionObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            const sectionName = entry.target.className || entry.target.id || 'unknown';
            trackEvent('section_view', {
                section: sectionName
            });
        }
    });
}, { threshold: 0.5 });

// Observe all main sections
document.querySelectorAll('section').forEach(section => {
    sectionObserver.observe(section);
});

// Add CSS for animations and mobile menu
const style = document.createElement('style');
style.textContent = `
    /* Animation styles */
    .hero__headline,
    .hero__subheadline,
    .hero__actions,
    .tunnel-visual,
    .value-prop,
    .step,
    .feature,
    .pricing-card,
    .download-card {
        opacity: 0;
        transform: translateY(30px);
        transition: all 0.6s cubic-bezier(0.16, 1, 0.3, 1);
    }
    
    .animate-in {
        opacity: 1;
        transform: translateY(0);
    }
    
    /* Mobile menu styles */
    @media (max-width: 768px) {
        .nav__menu {
            display: none;
        }
        
        .nav__menu--active {
            display: flex !important;
            position: fixed;
            top: 64px;
            left: 0;
            right: 0;
            background: rgba(10, 10, 10, 0.95);
            backdrop-filter: blur(20px);
            flex-direction: column;
            padding: var(--space-24);
            border-bottom: 1px solid var(--color-border);
            gap: var(--space-16);
            z-index: 999;
        }
    }
    
    .nav__toggle--active span:nth-child(1) {
        transform: rotate(45deg) translate(6px, 6px);
    }
    
    .nav__toggle--active span:nth-child(2) {
        opacity: 0;
    }
    
    .nav__toggle--active span:nth-child(3) {
        transform: rotate(-45deg) translate(6px, -6px);
    }
    
    /* Ripple effect */
    .btn {
        position: relative;
        overflow: hidden;
    }
    
    .ripple {
        position: absolute;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.4);
        transform: scale(0);
        animation: ripple-animation 0.6s linear;
        pointer-events: none;
    }
    
    @keyframes ripple-animation {
        to {
            transform: scale(4);
            opacity: 0;
        }
    }
    
    /* Header scroll effect */
    .header--scrolled {
        background: rgba(10, 10, 10, 0.95);
        backdrop-filter: blur(20px);
        box-shadow: 0 2px 20px rgba(0, 0, 0, 0.3);
    }
    
    /* Stagger animation delays */
    .value-prop:nth-child(1) { transition-delay: 0.1s; }
    .value-prop:nth-child(2) { transition-delay: 0.2s; }
    .value-prop:nth-child(3) { transition-delay: 0.3s; }
    
    .step:nth-child(1) { transition-delay: 0.1s; }
    .step:nth-child(2) { transition-delay: 0.2s; }
    .step:nth-child(3) { transition-delay: 0.3s; }
    .step:nth-child(4) { transition-delay: 0.4s; }
    
    .feature:nth-child(1) { transition-delay: 0.1s; }
    .feature:nth-child(2) { transition-delay: 0.2s; }
    .feature:nth-child(3) { transition-delay: 0.3s; }
    .feature:nth-child(4) { transition-delay: 0.4s; }
    .feature:nth-child(5) { transition-delay: 0.5s; }
    .feature:nth-child(6) { transition-delay: 0.6s; }
    
    .pricing-card:nth-child(1) { transition-delay: 0.1s; }
    .pricing-card:nth-child(2) { transition-delay: 0.2s; }
    .pricing-card:nth-child(3) { transition-delay: 0.3s; }
    
    .download-card:nth-child(1) { transition-delay: 0.1s; }
    .download-card:nth-child(2) { transition-delay: 0.2s; }
    .download-card:nth-child(3) { transition-delay: 0.3s; }
`;

document.head.appendChild(style);

// Console easter egg
console.log(`
╔══════════════════════════════════════╗
║                                      ║
║              tnnl.to                 ║
║       Remote Access Made Simple      ║
║                                      ║
║    Interested in our API?           ║
║    Check out our documentation      ║
║    at docs.tnnl.to                  ║
║                                      ║
╚══════════════════════════════════════╝
`);