package executionThreads;

import constants.Constants;

public class MainThread{	
	
	public static void main(String[] args) {
			new Hourglass();
	}
}

class Hourglass {
	HourglassSyncMethods monitor;
	TimerThread timerThread;
	PopupThread popupThread;
	Hourglass(){
		monitor = new HourglassSyncMethods();
		timerThread = new TimerThread(monitor);
		popupThread = new PopupThread(monitor);
	}
}


class TimerThread implements Runnable {
	HourglassSyncMethods g;
	
	public TimerThread(HourglassSyncMethods ins) {
		this.g = ins;
		new Thread(this, "TimerThread").start();
	}
	
	public void run() {
		while(true) {
			g.startTimer();
		}
	}
}

class PopupThread implements Runnable {
	HourglassSyncMethods g;
	
	public PopupThread(HourglassSyncMethods ins) {
		this.g = ins;
		new Thread(this, "PopupThread").start();
	}
	public void run() {
		while(true) {
			g.openWindow();
		}
	}
}


class HourglassSyncMethods{
	static boolean starterNode = true;
	Timer timer = new Timer();
	PopupWindow popup = new PopupWindow();
	
	//starter node
	public synchronized void startTimer() {
		if(!starterNode) {	//if popupWindow is not yet closed, wait until it closes and notifies
			try {
				wait();
				System.out.println("Waiting for popup window to receive input and close");
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		timer.startTimer();	//start timer after popupWindow has closed
		starterNode = false;//Don't forget to toggle this flag to prevent execution of this method again
		notifyAll();	//wake up the other thread after toggling flag
	}
	
	//other node
	public synchronized void openWindow() {
		if(starterNode) {	//if timer is yet to finish, wait until it notifies you it has
			try {
				wait();
				System.out.println("Waiting for timer to complete");
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		popup.openWindow();	//open window after timer has finished
		try {
			Thread.sleep(Constants.buttonDisableDuration);	//keep button disabled for specified time
		} catch (InterruptedException e) {
			e.printStackTrace();
		}
		Constants.disabledButton.setEnabled(true);	//enable button after time has elapsed
		while(Constants.continueTimer == false) {
			try {
				Thread.sleep(1000);					//check every second if user's input translates to 'continue the timer'
													//The other input directly terminates the program
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		popup.closeWindow();//After receiving input, close window
		Constants.continueTimer = false;	//reset user input choice
		starterNode = true;	//toggle flag to prevent popupWindow from opening up immediately
		notifyAll();	//wake up timer thread to begin its execution after toggling flag
	}
}
